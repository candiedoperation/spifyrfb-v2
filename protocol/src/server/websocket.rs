/*
    SpifyRFB - Modern RFB Server implementation using Rust
    Copyright (C) 2023  Atheesh Thirumalairajan

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use crate::{debug, server::{parser, ipc_client}, win32};
use std::{error::Error, time::Duration, sync::Arc, pin::Pin, process, env, fs};
use super::{parser::{websocket::OPCODE, GetBits}, FrameBufferUpdate, WindowManager, RFBEncodingType};
use rustls::ServerConfig;
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt, AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
    time::timeout,
};
use tokio_rustls::{TlsAcceptor, server::TlsStream};

pub struct WSCreateOptions {
    pub(crate) tcp_address: String, 
    pub(crate) proxy_address: String, 
    pub(crate) secure: bool,
    pub(crate) spify_daemon: bool
}

enum WebsocketStream {
    WS(TcpStream),
    WSS(TlsStream<TcpStream>)
}

impl AsyncRead for WebsocketStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            WebsocketStream::WS(stream) => { Pin::new(stream).poll_read(cx, buf) },
            WebsocketStream::WSS(stream) => { Pin::new(stream).poll_read(cx, buf) },
        }
    }
}

impl AsyncWrite for WebsocketStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match &mut *self {
            WebsocketStream::WS(stream) => { Pin::new(stream).poll_write(cx, buf) },
            WebsocketStream::WSS(stream) => { Pin::new(stream).poll_write(cx, buf) },
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), std::io::Error>> {
        match &mut *self {
            WebsocketStream::WS(stream) => { Pin::new(stream).poll_flush(cx) },
            WebsocketStream::WSS(stream) => { Pin::new(stream).poll_flush(cx) },
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), std::io::Error>> {
        match &mut *self {
            WebsocketStream::WS(stream) => { Pin::new(stream).poll_shutdown(cx) },
            WebsocketStream::WSS(stream) => { Pin::new(stream).poll_shutdown(cx) },
        }
    }
}

async fn proxy_websocket(ws_stream: WebsocketStream, proxy_address: String) {
    /* Split Stream for simulatneous TX/RX */
    let (mut client_rx, mut client_tx) = io::split(ws_stream);
    let mut pending_writes: Vec<Vec<u8>> = vec![];
    let mut fin_payload: Vec<u8> = vec![];
    let mut fin_payload_opcode: u8 = 0;

    /* Connect to Remote Host */
    let remote_connection = TcpStream::connect(proxy_address).await;
    if remote_connection.is_err() {
        /* Disconnect from Client, Server Error(1011) */
        let mut frame_payload = 1011_u16.to_be_bytes().to_vec();
        frame_payload.extend_from_slice("Remote Host Connection Failed".as_bytes());

        /* Generate Frame */
        let frame = parser::websocket::create_frame(
            frame_payload,
            OPCODE::CONNECTION_CLOSE,
            false,
        );

        /* Push to pending writes and init Remote */
        pending_writes.push(frame);
        return;
    }

    let mut remote = remote_connection.unwrap();
    loop {
        /* Read Websocket Opcode */
        let mut buf: [u8; 2] = [0; 2];
        let rx_timeout = timeout(
            Duration::from_millis(10), 
            client_rx.read_exact(&mut buf)
        ).await;

        if rx_timeout.is_ok() {
            let rx = rx_timeout.unwrap();
            if rx.unwrap_or(0) != 0 {
                /*
                    We create slices using a range within brackets by specifying
                    [starting_index..ending_index], where starting_index is the
                    first position in the slice and ending_index is one more than
                    the last position in the slice.
                */

                /* Get Opcode */
                let fin_flag = buf[0].get_bits_le()[0];
                let mut opcode: u8 = u8::from_bits(buf[0].get_bits_le()[4..8].to_vec(), true);

                /* Find Payload Hint */
                let payload_length: u64;
                let mask_key: Option<[u8; 4]>;
                let mask_hint: bool = buf[1].get_bits_le()[0];
                let payload_hint = u8::from_bits(buf[1].get_bits_le()[1..8].to_vec(), true);

                if payload_hint < 126 {
                    /* Payload length is same as hint (if = or < 125) */
                    payload_length = payload_hint as u64;
                } else if payload_hint == 126 {
                    /* Read next 16 bits */
                    let mut payload_buf: [u8; 2] = [0; 2];
                    client_rx.read_exact(&mut payload_buf).await.unwrap();
                    payload_length = u16::from_be_bytes(payload_buf) as u64;
                } else if payload_hint == 127 {
                    /* Read next 64 bits */
                    let mut payload_buf: [u8; 8] = [0; 8];
                    client_rx.read_exact(&mut payload_buf).await.unwrap();
                    payload_length = u64::from_be_bytes(payload_buf);
                } else {
                    payload_length = 0;
                }

                if mask_hint == true {
                    /* Next 32 bits is Mask Key */
                    let mut mask_key_buf: [u8; 4] = [0; 4];
                    client_rx.read_exact(&mut mask_key_buf).await.unwrap();
                    mask_key = Option::Some(mask_key_buf);
                } else {
                    /* Set Mask Key to None */
                    mask_key = Option::None;

                    /* Initiate Disconnect, Policy Violation(1008) */
                    let mut frame_payload = 1008_u16.to_be_bytes().to_vec();
                    frame_payload.extend_from_slice("Payload is not Masked".as_bytes());

                    /* Construct Frame */
                    let frame = parser::websocket::create_frame(
                        frame_payload,
                        OPCODE::CONNECTION_CLOSE,
                        false,
                    );

                    /* Push to pending writes */
                    pending_writes.push(frame);
                }

                /* Read the Payload */
                let mut payload: Vec<u8> = vec![0; payload_length as usize];
                let payload_buffer = &mut payload[..];
                client_rx.read_exact(payload_buffer).await.unwrap();

                /* Decode the Payload, if mask was set */
                if mask_key.is_some() {
                    let payload_mask = mask_key.unwrap();
                    payload = parser::websocket::unmask_payload(payload_mask, payload);
                }

                if fin_flag == false {
                    /* Extend FIN Payload */
                    if opcode != OPCODE::CONTINUATION_FRAME {
                        fin_payload_opcode = opcode;
                    }
                    fin_payload.extend_from_slice(&payload[..]);

                    /* Do not process the Payload */
                    continue;
                } else if fin_flag == true && opcode == OPCODE::CONTINUATION_FRAME {
                    fin_payload.extend_from_slice(&payload[..]);
                    payload = fin_payload.clone();
                    opcode = fin_payload_opcode;

                    /* Clear FIN Payload for next FIN Message */
                    fin_payload = vec![];
                }

                /* Process the Payload */
                match opcode {
                    OPCODE::TEXT_FRAME => {
                        /* Write Payload to Remote Host */
                        remote.write_all(&payload[..]).await.unwrap();
                    },
                    OPCODE::BINARY_FRAME => {
                        /* Write Payload to Remote Host */
                        remote.write_all(&payload[..]).await.unwrap();
                    },
                    OPCODE::PING => {
                        /* Create PONG Frame */
                        let frame = parser::websocket::create_frame(
                            "Pong".as_bytes().to_vec(),
                            OPCODE::PONG,
                            false,
                        );

                        /* Push to pending writes */
                        pending_writes.push(frame);
                    }
                    OPCODE::CONNECTION_CLOSE => {
                        let remote_shutdown = remote.shutdown().await;
                        let mut frame_payload: Vec<u8>;
                        if remote_shutdown.is_err() {
                            /* 1011 -> Server Error */
                            frame_payload = 1011_u16.to_be_bytes().to_vec();
                            frame_payload.extend_from_slice("Failed to Close Remote Connection".as_bytes());
                        } else {
                            /* 1000 -> Normal Closure */
                            frame_payload = 1000_u16.to_be_bytes().to_vec();
                            frame_payload.extend_from_slice("Remote Connection Closed".as_bytes());
                        }

                        let frame = parser::websocket::create_frame(
                            frame_payload,
                            OPCODE::CONNECTION_CLOSE,
                            false,
                        );

                        /* Push to pending writes */
                        pending_writes.push(frame);
                    }
                    _ => {
                        debug::l1(format!("Invalid OPCODE: {}", opcode));
                    }
                }
            } else {
                /* Failed to Reach Client, Close Remote Host Connection */
                remote.shutdown().await.unwrap();
                return;
            }
        } else {
            /* Read from Remote Host and Write to Websocket Client */
            let mut remote_buffer: Vec<u8> = vec![0; 32768];
            let remote_rx_timeout = timeout(
                Duration::from_millis(10),
                remote.read(&mut remote_buffer)
            ).await;

            if remote_rx_timeout.is_ok() {
                let remote_rx = remote_rx_timeout.unwrap();
                let frame = parser::websocket::create_frame(
                    remote_buffer[..remote_rx.unwrap()].to_vec(),
                    OPCODE::BINARY_FRAME,
                    false,
                );
                
                /* Push to Pending Writes */
                pending_writes.push(frame);
            }

            /* Send all Pending Writes */
            for ws_payload in pending_writes {
                client_tx
                .write_all(&ws_payload)
                .await
                .unwrap();
            }

            /* Reset Pending Writes */
            pending_writes = vec![];
        }
    }
}

async fn handle_wsclient(mut ws_stream: WebsocketStream, proxy_address: String) {
    let mut buf: [u8; 32768] = [0; 32768];
    let bits_read = ws_stream.read(&mut buf).await.unwrap();

    let handshake_request = String::from_utf8_lossy(&buf[..bits_read]);
    let handshake_request: Vec<&str> = handshake_request.split("\r\n").collect();

    /* Debugging */
    //debug::l1(format!("Request: {:?}", handshake_request));

    let handshake_request_version = parser::http::get_version(handshake_request.clone());
    let handshake_request_method = parser::http::get_method(handshake_request.clone());
    let handshake_websocket_version: u8 = parser::http::get_websocket_version(handshake_request.clone());
    let valid_websocket_version = handshake_websocket_version == 13; /* Use Array in Future Impl. */

    if handshake_request_version == "HTTP/1.1" && handshake_request_method == "GET" && valid_websocket_version == true {
        /* This is a valid Websocket Handshake Request, Check WS Version Support */
        let websocket_key = parser::http::get_websocket_key(handshake_request.clone());
        let websocket_accept_key = parser::websocket::get_accept_key(websocket_key);

        /* Generate Handshake Response */
        let handshake_response = parser::http::response_from_headers(
            [
                "HTTP/1.1 101 Switching Protocols",
                "Upgrade: websocket",
                "Connection: Upgrade",
                format!("Sec-WebSocket-Accept: {}", websocket_accept_key).as_str(),
            ]
            .to_vec()
        );

        /* Send Response and Complete Handshake */
        ws_stream.write(handshake_response.as_bytes()).await.unwrap();
        ws_stream.write(b"\r\n").await.unwrap();

        /* Handshake Response Sent, Proceed Further */
        proxy_websocket(ws_stream, proxy_address).await;
    } else {
        if handshake_websocket_version == 0 {
            /* This is not a Websocket Upgrade Request: See parser.rs */
            let api_response = get_api_response(parser::http::get_request_uri(handshake_request.clone()));
            ws_stream.write_all(api_response.0.as_bytes()).await.unwrap();
            ws_stream.write_all("\r\n".as_bytes()).await.unwrap();
            ws_stream.write_all(&api_response.1).await.unwrap();
        } else {
            /* Send 400 (Bad Request) */
            let response_message = "Websocket/HTTP Versions Unsupported";
            let handshake_response = parser::http::response_from_headers(
                [
                    "HTTP/1.1 400 Bad Request",
                    format!("Content-length: {}", response_message.as_bytes().len()).as_str(),
                    "Content-type: text/plain",
                    "\n",
                    response_message
                ]
                .to_vec()
            );
    
            /* Send Response and Complete Handshake */
            ws_stream.write_all(handshake_response.as_bytes()).await.unwrap();
            ws_stream.write(b"\r\n").await.unwrap();
        }
    }
}

fn get_api_response(uri: (String, String)) -> (String, Vec<u8>) {
    let mut payload: Vec<u8> = vec![];
    let default_message = format!("{} was not found  ", uri.1);
    let mut api_response: String = parser::http::response_from_headers(
        [
            "HTTP/1.1 404 Not Found",
            format!("Content-length: {}", default_message.as_bytes().len()).as_str(),
            "Content-type: text/plain",
            "\n",
            default_message.as_str()
        ].to_vec()
    );

    if uri.0 == "GET" {
        if uri.1 == "/" {
            let response_message = "<h1>SpifyRFB Websocket Service</h1><p>Apps like noVNC can interpret this page</p> ";
            api_response = parser::http::response_from_headers(
                [
                    "HTTP/1.1 200 OK",
                    format!("Content-length: {}", response_message.as_bytes().len()).as_str(),
                    "Content-type: text/html",
                    "\n",
                    response_message
                ]
                .to_vec()
            );
        } else if uri.1.starts_with("/api/power") == true {
            let mut status: bool = false;
            if uri.1 == "/api/power/lock" {
                #[cfg(target_os = "windows")]
                { status = win32::lock_workstation(); }
            } else if uri.1 == "/api/power/logoff" {
                #[cfg(target_os = "windows")]
                { status = win32::logoff() }
            } else if uri.1 == "/api/power/shutdown" {
                #[cfg(target_os = "windows")]
                { status = win32::shutdown() }
            } else if uri.1 == "/api/power/reboot" {
                #[cfg(target_os = "windows")]
                { status = win32::restart() }
            }

            let status = format!("{:?}", status);
            api_response = parser::http::response_from_headers(
                [
                    "HTTP/1.1 200 OK",
                    "Content-type: text/plain",
                    "\n",
                    &status
                ]
                .to_vec()
            );
        } else if uri.1 == "/api/screenshot" {
            /* Verify Client Auth in Future */
            let mut framebufferupdate: FrameBufferUpdate = Default::default();

            #[cfg(target_os = "windows")]
            {
                let win32_connection = win32::connect(true);
                if win32_connection.is_ok() {
                    let wm_arc = win32_connection.unwrap();
                    match wm_arc.as_ref() {
                        WindowManager::WIN32(win32_server) => {
                            let primary_display = win32_server.monitors[0].clone();
                            framebufferupdate = win32::rectangle_framebuffer_update(
                                win32_server, 
                                primary_display.clone(), 
                                RFBEncodingType::RAW, 
                                0, 
                                0, 
                                primary_display.monitor_devmode.dmPelsWidth as u16, 
                                primary_display.monitor_devmode.dmPelsHeight as u16, 
                                String::from("webapi")
                            );
                        }
                    }
                }
            }

            let framebuffer_rect = &framebufferupdate.frame_buffer[0];            
            let mut png_data: Vec<u8> = Vec::new();

            let mut encoder = png::Encoder::new(
                &mut png_data, 
                framebuffer_rect.width as u32, 
                framebuffer_rect.height as u32
            );

            /* Set Encoder Parameters */
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);

            /* Define a PngWriter */
            let mut png_writer = encoder.write_header().unwrap();
            png_writer.write_image_data(&framebuffer_rect.encoded_pixels.clone()).unwrap();
            png_writer.finish().unwrap();

            /* Update */
            payload = png_data;
            api_response = parser::http::response_from_headers([
                "HTTP/1.1 200 OK",
                format!("Content-length: {}", payload.len()).as_str(),
                "Content-type: image/png",
            ].to_vec());
        }
    }

    /* Return API Response */
    (api_response, payload)
}

pub async fn create(options: WSCreateOptions) -> Result<(), Box<dyn Error>> {
    match TcpListener::bind(options.tcp_address).await {
        Ok(listener) => {
            let ws_address = listener.local_addr().unwrap();
            println!("SpifyRFB Websocket Communications at {:?}\n", ws_address);

            if options.spify_daemon {
                /* Send IP Address Update to Daemon */
                ipc_client::send_ip_update(
                    format!(
                        "{}\r\nws{}\r\n{}", 
                        process::id().to_string(),
                        if options.secure == true { "s" } else { "" }, 
                        ws_address
                    )
                ).await;
            }

            /* Define TLS Objects */
            let tls_serverconfig: Option<ServerConfig>;
            let mut tls_acceptor: Option<TlsAcceptor> = Option::None;

            if options.secure == true {
                let mut spify_installpath = env::current_exe().unwrap();
                spify_installpath.pop();
                spify_installpath.push("ssl");
                spify_installpath.push("cert");

                /* 
                    We added an extra directory 'cert' as
                    Rust pops the last element in the path
                    when set_file_name() is invoked.
                */

                spify_installpath.set_file_name("cert.pem");
                let certificate_path = spify_installpath.clone();
                let certificate_path = certificate_path.to_str().unwrap();

                spify_installpath.set_file_name("key.pem");
                let key_path = spify_installpath.clone();
                let key_path = key_path.to_str().unwrap();

                tls_serverconfig = Option::Some(
                    ServerConfig::builder()
                    .with_safe_defaults()
                    .with_no_client_auth()
                    .with_single_cert(
                        parser::tls::load_certificates(certificate_path),
                        parser::tls::load_privatekey(key_path)
                    )
                    .unwrap()
                );

                tls_acceptor = Option::Some(
                    TlsAcceptor::from(Arc::new(tls_serverconfig.clone().unwrap()))
                );
            }

            loop {
                /* Define Spawn Requirements */
                let proxyaddr = options.proxy_address.clone();
                let (client, _) = listener.accept().await?;
                let tls_acceptor = tls_acceptor.clone();

                tokio::spawn(async move {
                    /* Init Handshake */
                    println!("HTTP/WS Connection Established: {:?}", client);

                    let ws_stream: WebsocketStream;
                    if tls_acceptor.is_some() {
                        ws_stream = WebsocketStream::WSS(
                            tls_acceptor.unwrap().accept(client).await.unwrap()
                        );
                    } else {
                        ws_stream = WebsocketStream::WS(client);
                    }

                    handle_wsclient(
                        ws_stream, 
                        proxyaddr
                    ).await;
                });
            }
        }
        Err(err) => {
            println!("Websocket IP Address Binding Failed -> {}", err.to_string());
            Err(err.into())
        }
    }
}
