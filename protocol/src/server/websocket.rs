use crate::{debug, server::parser};
use std::{error::Error, time::Duration};
use super::parser::{websocket::OPCODE, GetBits};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};

async fn proxy_websocket(client: TcpStream, proxy_address: String) {
    /* Split Stream for simulatneous TX/RX */
    let (mut client_rx, mut client_tx) = io::split(client);
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

async fn handle_wsclient(mut client: TcpStream, proxy_address: String) {
    let mut buf: [u8; 32768] = [0; 32768];
    let bits_read = client.read(&mut buf).await.unwrap();

    let handshake_request = String::from_utf8_lossy(&buf[..bits_read]);
    let handshake_request: Vec<&str> = handshake_request.split("\r\n").collect();

    /* Debugging */
    //println!("Request: {:?}", handshake_request);

    let handshake_request_version = parser::http::get_version(handshake_request.clone());
    let handshake_request_method = parser::http::get_method(handshake_request.clone());
    if handshake_request_version == "HTTP/1.1" && handshake_request_method == "GET" {
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
            .to_vec(),
        );

        /* Send Response and Complete Handshake */
        client.write(handshake_response.as_bytes()).await.unwrap();

        /* Handshake Response Sent, Proceed Further */
        proxy_websocket(client, proxy_address).await;
    } else {
        /* Send 400 (Bad Request) */
    }
}

pub async fn create(tcp_address: String, proxy_address: String) -> Result<(), Box<dyn Error>> {
    match TcpListener::bind(tcp_address).await {
        Ok(listener) => {
            println!(
                "SpifyRFB Websocket Communications at {:?}\n",
                listener.local_addr().unwrap()
            );

            loop {
                /* Define Spawn Requirements */
                let (client, _) = listener.accept().await?;
                let proxyaddr = proxy_address.clone();

                tokio::spawn(async move {
                    /* Init Handshake */
                    println!("Connection Established: {:?}", client);
                    handle_wsclient(client, proxyaddr).await;
                });
            }
        }
        Err(err) => {
            println!("Websocket IP Address Binding Failed -> {}", err.to_string());
            Err(err.into())
        }
    }
}
