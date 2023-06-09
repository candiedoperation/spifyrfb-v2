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

pub mod encoding_raw;
pub mod encoding_zrle;
pub mod encoding_zlib;
pub mod encoding_hextile;
pub mod websocket;
pub mod parser;
pub mod ipc_client;

use crate::{server::{parser::GetBits, websocket::WSCreateOptions}, debug};

#[cfg(target_os = "windows")]
use crate::win32;

#[cfg(target_os = "linux")]
use crate::x11;

use std::{error::Error, sync::Arc, process};
use des::{Des, cipher::{KeyInit, generic_array::GenericArray, typenum, BlockDecrypt}};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{ReadHalf, WriteHalf},
        TcpListener, TcpStream,
    },
};
use uuid::Uuid;

pub struct CreateOptions {
    pub ip_address: String, 
    pub ws_proxy: Option<(String, bool)>, 
    pub auth: Option<RFBAuthentication>,
    pub spify_daemon: bool
}

struct ClientToServerMessage;
impl ClientToServerMessage {
    const SET_PIXEL_FORMAT: u8 = 0;
    const SET_ENCODINGS: u8 = 2;
    const FRAME_BUFFER_UPDATE_REQUEST: u8 = 3;
    const KEY_EVENT: u8 = 4;
    const POINTER_EVENT: u8 = 5;
    const CLIENT_CUT_TEXT: u8 = 6;
}

pub struct ServerToClientMessage;
impl ServerToClientMessage {
    pub const FRAME_BUFFER_UPDATE: u8 = 0;
    pub const SET_COLOR_MAP_ENTRIES: u8 = 1;
    pub const BELL: u8 = 2;
    pub const SERVER_CUT_TEXT: u8 = 3;
}

struct RFBError {
    reason_length: u32,
    reason_string: String,
}

#[derive(Clone)]
pub struct VNCAuth {
    pub security_key: [u8; 8]
}

#[derive(Clone)]
pub enum RFBAuthentication {
    Vnc(VNCAuth)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PixelFormat {
    pub(crate) bits_per_pixel: u8,
    pub(crate) depth: u8,
    pub(crate) big_endian_flag: u8,
    pub(crate) true_color_flag: u8,
    pub(crate) red_max: u16,
    pub(crate) green_max: u16,
    pub(crate) blue_max: u16,
    pub(crate) red_shift: u8,
    pub(crate) green_shift: u8,
    pub(crate) blue_shift: u8,
    pub(crate) padding: [u8; 3], /* THREE BYTES */
}

#[derive(Debug)]
pub struct RFBServerInit {
    pub(crate) framebuffer_width: u16,
    pub(crate) framebuffer_height: u16,
    pub(crate) server_pixelformat: PixelFormat,
    pub(crate) name_length: u32,
    pub(crate) name_string: String,
}

pub struct RFBEncodingType;
impl RFBEncodingType {
    pub const RAW: i32 = 0;
    pub const COPY_RECT: i32 = 1;
    pub const RRE: i32 = 2;
    pub const HEX_TILE: i32 = 5;
    pub const ZLIB: i32 = 6;
    pub const TIGHT: i32 = 7;
    pub const TRLE: i32 = 15;
    pub const ZRLE: i32 = 16;
}

#[derive(Debug)]
pub struct FrameBufferRectangle {
    pub(crate) x_position: u16,
    pub(crate) y_position: u16,
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) encoding_type: i32,
    pub(crate) encoded_pixels: Vec<u8>,
    pub(crate) encoded_pixels_length: u32
}

#[derive(Clone)]
pub struct FrameBuffer {
    pub(crate) x_position: u16,
    pub(crate) y_position: u16,
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) bits_per_pixel: u8,
    pub(crate) encoding: i32,
    pub(crate) raw_pixels: Vec<u8>,
    pub(crate) encoded_pixels: Vec<u8>
}

#[derive(Debug, Default)]
pub struct FrameBufferUpdate {
    pub(crate) message_type: u8,
    pub(crate) padding: u8,
    pub(crate) number_of_rectangles: u16,
    pub(crate) frame_buffer: Vec<FrameBufferRectangle>,
}

pub enum WindowManager {
    #[cfg(target_os = "linux")]
    X11(x11::X11Server),
    /* Add Wayland in Future */

    #[cfg(target_os = "windows")]
    WIN32(win32::Win32Server),
}

struct RFBServer {
    protocol_version: [u8; 12],
    supported_security_types_length: u8,
    supported_security_types: Vec<u8>,
}

impl RFBServer {
    fn init() -> RFBServer {
        RFBServer {
            protocol_version: String::from("RFB 003.008\n").as_bytes().try_into().unwrap(),
            supported_security_types_length: 1,
            supported_security_types: vec![1], /* SECURITY TYPE 0 IS INVALID */
        }
    }
}

fn create_rfb_error(reason_string: String) -> RFBError {
    /* CREATES A STANDARD RFB_ERROR MESSAGE */
    RFBError {
        reason_length: reason_string.len().try_into().unwrap(),
        reason_string,
    }
}

async fn write_framebuffer_update_message(
    client_tx: &mut WriteHalf<'_>,
    frame_buffer: FrameBufferUpdate,
) {
    client_tx
        .write_u8(frame_buffer.message_type)
        .await
        .unwrap_or(());
    client_tx.write_u8(frame_buffer.padding).await.unwrap_or(());
    client_tx
        .write_u16(frame_buffer.number_of_rectangles)
        .await
        .unwrap_or(());

    for framebuffer in frame_buffer.frame_buffer {
        client_tx
            .write_u16(framebuffer.x_position)
            .await
            .unwrap_or(());
        client_tx
            .write_u16(framebuffer.y_position)
            .await
            .unwrap_or(());
        client_tx.write_u16(framebuffer.width).await.unwrap_or(());
        client_tx.write_u16(framebuffer.height).await.unwrap_or(());
        client_tx
            .write_i32(framebuffer.encoding_type)
            .await
            .unwrap_or(());
        
        match framebuffer.encoding_type {
            RFBEncodingType::ZRLE => {
                client_tx
                .write_u32(framebuffer.encoded_pixels_length)
                .await
                .unwrap();

                client_tx
                .write_all(framebuffer.encoded_pixels.as_slice())
                .await
                .unwrap();
            },
            RFBEncodingType::ZLIB => {
                client_tx
                .write_u32(framebuffer.encoded_pixels_length)
                .await
                .unwrap();

                client_tx
                .write_all(framebuffer.encoded_pixels.as_slice())
                .await
                .unwrap();
            },
            _ => {
                client_tx
                .write_all(framebuffer.encoded_pixels.as_slice())
                .await
                .unwrap_or(());
            }
        }
    }

    //debug::l1(format!("FBU Response Time: {:?}", debug::time_now()));
}

async fn process_clientserver_message(
    _client_rx: &mut ReadHalf<'_>,
    client_tx: &mut WriteHalf<'_>,
    opcode: &[u8],
    buffer: &[u8],
    pixelformat: PixelFormat,
    zstream_id: String,
    wm: Arc<WindowManager>
) {
    match opcode[0] {
        ClientToServerMessage::SET_PIXEL_FORMAT => {
            /* Send Framebuffer Update */
            match wm.as_ref() {
                #[cfg(target_os = "windows")]
                WindowManager::WIN32(win32_server) => {
                    let win32_monitor = win32_server.monitors[0].clone();
                    write_framebuffer_update_message(
                        client_tx,
                        win32::rectangle_framebuffer_update(
                            win32_server,
                            win32_monitor.clone(),
                            RFBEncodingType::ZRLE,
                            0,
                            0,
                            win32_monitor.monitor_devmode.dmPelsWidth as u16,
                            win32_monitor.monitor_devmode.dmPelsHeight as u16,
                            pixelformat,
                            zstream_id
                        ),
                    )
                    .await;
                },
                #[cfg(target_os = "linux")]
                WindowManager::X11(x11_server) => {
                    let x11_screen = x11_server.displays[0].clone();
                    write_framebuffer_update_message(
                        client_tx,
                        x11::rectangle_framebuffer_update(
                            &x11_server,
                            x11_screen.clone(),
                            RFBEncodingType::ZRLE,
                            0,
                            0,
                            x11_screen.width_in_pixels,
                            x11_screen.height_in_pixels,
                            pixelformat,
                            zstream_id
                        ),
                    )
                    .await;
                }
            }
        }
        ClientToServerMessage::SET_ENCODINGS => {
            debug::l1(format!("Set Encodings Request"));
        }
        ClientToServerMessage::FRAME_BUFFER_UPDATE_REQUEST => {
            /* let incremental: u8 = message[0]; */
            let x_position: u16 = ((buffer[1] as u16) << 8) | buffer[2] as u16;
            let y_position: u16 = ((buffer[3] as u16) << 8) | buffer[4] as u16;
            let width: u16 = ((buffer[5] as u16) << 8) | buffer[6] as u16;
            let height: u16 = ((buffer[7] as u16) << 8) | buffer[8] as u16;

            match wm.as_ref() {
                #[cfg(target_os = "windows")]
                WindowManager::WIN32(win32_server) => {
                    write_framebuffer_update_message(
                        client_tx,
                        win32::rectangle_framebuffer_update(
                            win32_server,
                            win32_server.monitors[0].clone(),
                            RFBEncodingType::ZRLE,
                            x_position as i16,
                            y_position as i16,
                            width,
                            height,
                            pixelformat,
                            zstream_id
                        ),
                    )
                    .await;
                },
                #[cfg(target_os = "linux")]
                WindowManager::X11(x11_server) => {
                    write_framebuffer_update_message(
                        client_tx,
                        x11::rectangle_framebuffer_update(
                            &x11_server,
                            x11_server.displays[0].clone(),
                            RFBEncodingType::ZRLE,
                            x_position as i16,
                            y_position as i16,
                            width,
                            height,
                            pixelformat,
                            zstream_id
                        ),
                    )
                    .await;
                }
            }
        }
        ClientToServerMessage::POINTER_EVENT => match wm.as_ref() {
            #[cfg(target_os = "windows")]
            WindowManager::WIN32(win32_server) => {
                let button_mask = buffer[0];
                let dst_x = (((buffer[1] as u16) << 8) | buffer[2] as u16)
                    .try_into()
                    .unwrap_or(0);
                let dst_y = (((buffer[3] as u16) << 8) | buffer[4] as u16)
                    .try_into()
                    .unwrap_or(0);

                win32::fire_pointer_event(win32::Win32PointerEvent { 
                    dst_x,
                    dst_y,
                    button_mask
                }, win32_server.monitors[0].clone());
            },
            #[cfg(target_os = "linux")]
            WindowManager::X11(x11_server) => {
                let mut button_mask = buffer[0];
                let dst_x = (((buffer[1] as u16) << 8) | buffer[2] as u16)
                    .try_into()
                    .unwrap_or(0);
                let dst_y = (((buffer[3] as u16) << 8) | buffer[4] as u16)
                    .try_into()
                    .unwrap_or(0);

                /*
                    RFB BUTTON MASKS (Observed):
                    BUTTON_UP:     0b00000000 = 0d0
                    BUTTON_LEFT:   0b00000001 = 0d1
                    BUTTON_MIDDLE: 0b00000010 = 0d2
                    BUTTON_RIGHT:  0b00000100 = 0d4
                    BTN_SCROLLUP:  0b00001000 = 0d8
                    BTN_SCROLLDN:  0b00010000 = 0d16
                */

                button_mask = match button_mask {
                    0 => 0,
                    1 => 1,
                    2 => 2,
                    4 => 3,
                    8 => 4,
                    16 => 5,
                    _ => 0,
                };

                let x11_pointer_event = x11::X11PointerEvent {
                    dst_x,
                    dst_y,
                    button_mask,
                };

                x11::fire_pointer_event(
                    x11_server,
                    x11_server.displays[0].clone(),
                    x11_pointer_event,
                );
            }
        },
        ClientToServerMessage::KEY_EVENT => {
            let down_flag: u8 = buffer[0];
            let key_sym: u32 = (buffer[3] as u32) << 24
                | (buffer[4] as u32) << 16
                | (buffer[5] as u32) << 8
                | (buffer[6] as u32);

            match wm.as_ref() {
                #[cfg(target_os = "windows")]
                WindowManager::WIN32(win32_server) => {
                    /* SEND WIN32 KEYPRESS EVENT */
                    win32::fire_key_event(win32_server, key_sym, down_flag);
                },
                #[cfg(target_os = "linux")]
                WindowManager::X11(x11_server) => {
                    x11::fire_key_event(
                        &x11_server, 
                        x11_server.displays[0].clone(), 
                        x11::X11KeyEvent {
                            key_down: down_flag,
                            key_sym,
                        }
                    );
                }
            }
        }
        ClientToServerMessage::CLIENT_CUT_TEXT => {}
        _ => {}
    }
}

async fn init_clientserver_handshake(mut client: TcpStream, wm: Arc<WindowManager>) {
    /* Session Statics */
    let (mut client_rx, mut client_tx) = client.split();

    /* Create Endpoint Specific ZLib Stream, PixelFormat */
    let zstream_id = Uuid::new_v4().to_string();
    encoding_zlib::create_stream(zstream_id.clone());
    
    #[allow(unused_assignments)]
    let mut pixel_format: PixelFormat = Default::default();

    #[cfg(target_os = "windows")]
    { pixel_format = win32::get_pixelformat() }

    #[cfg(target_os = "linux")]
    {
        match wm.as_ref() {
            WindowManager::X11(x11_server) => {
                pixel_format = x11::get_pixelformat(
                    x11_server.displays[0].clone()
                );
            },
        }
    }

    loop {
        let mut opcode: [u8; 1] = [0; 1];
        let payload_result = client_rx.read_exact(&mut opcode).await;
        if payload_result.unwrap_or(0) != 0 {
            match opcode[0] {
                ClientToServerMessage::SET_PIXEL_FORMAT => {
                    let mut buffer: [u8; 19] = [0; 19];
                    client_rx.read_exact(&mut buffer).await.unwrap();
                    
                    /* Check if first three bytes are padding */
                    if buffer[0] == 0 && buffer[1] == 0 && buffer[2] == 0 {
                        let pfu = &buffer[3..];
                        pixel_format = PixelFormat {
                            bits_per_pixel: pfu[0],
                            depth: pfu[1],
                            big_endian_flag: pfu[2],
                            true_color_flag: pfu[3],
                            red_max: (pfu[4] as u16) << 8 | (pfu[5] as u16),
                            green_max: (pfu[6] as u16) << 8 | (pfu[7] as u16),
                            blue_max: (pfu[8] as u16) << 8 | (pfu[9] as u16),
                            red_shift: pfu[10],
                            green_shift: pfu[11],
                            blue_shift: pfu[12],
                            padding: [0, 0, 0],
                        };
                    } 

                    process_clientserver_message(
                        &mut client_rx, 
                        &mut client_tx, 
                        &opcode, 
                        &buffer, 
                        pixel_format,
                        zstream_id.clone(), 
                        wm.clone()
                    )
                    .await;
                }
                ClientToServerMessage::SET_ENCODINGS => {
                    let mut buffer: [u8; 3] = [0; 3];
                    client_rx.read_exact(&mut buffer).await.unwrap();
                    process_clientserver_message(
                        &mut client_rx,
                        &mut client_tx,
                        &opcode,
                        &buffer,
                        pixel_format,
                        zstream_id.clone(),
                        wm.clone()
                    )
                    .await;
                }
                ClientToServerMessage::FRAME_BUFFER_UPDATE_REQUEST => {
                    //debug::l1(format!("FBU Request Time: {:?}", debug::time_now()));
                    let mut buffer: [u8; 9] = [0; 9];
                    client_rx.read_exact(&mut buffer).await.unwrap();
                    process_clientserver_message(
                        &mut client_rx,
                        &mut client_tx,
                        &opcode,
                        &buffer,
                        pixel_format,
                        zstream_id.clone(),
                        wm.clone()
                    )
                    .await;
                }
                ClientToServerMessage::POINTER_EVENT => {
                    let mut buffer: [u8; 5] = [0; 5];
                    client_rx.read_exact(&mut buffer).await.unwrap();
                    process_clientserver_message(
                        &mut client_rx,
                        &mut client_tx,
                        &opcode,
                        &buffer,
                        pixel_format,
                        zstream_id.clone(),
                        wm.clone()
                    )
                    .await;
                }
                ClientToServerMessage::KEY_EVENT => {
                    let mut buffer: [u8; 7] = [0; 7];
                    client_rx.read_exact(&mut buffer).await.unwrap();
                    process_clientserver_message(
                        &mut client_rx,
                        &mut client_tx,
                        &opcode,
                        &buffer,
                        pixel_format.clone(),
                        zstream_id.clone(),
                        wm.clone()
                    )
                    .await;
                }
                _ => { /* EXCEPTION EVENT: CLIENT_CUT_TEXT */ }
            }
        } else {
            encoding_zlib::flush_stream(zstream_id.clone());
            debug::l1(format!("Client Has Disconnected"));
            break;
        }
    }
}

async fn write_serverinit_message(
    mut client: TcpStream,
    server_init: RFBServerInit,
    wm: Arc<WindowManager>,
) {
    client
        .write_u16(server_init.framebuffer_width)
        .await
        .unwrap_or(());
    client
        .write_u16(server_init.framebuffer_height)
        .await
        .unwrap_or(());
    client
        .write_u8(server_init.server_pixelformat.bits_per_pixel)
        .await
        .unwrap_or(());
    client
        .write_u8(server_init.server_pixelformat.depth)
        .await
        .unwrap_or(());
    client
        .write_u8(server_init.server_pixelformat.big_endian_flag)
        .await
        .unwrap_or(());
    client
        .write_u8(server_init.server_pixelformat.true_color_flag)
        .await
        .unwrap_or(());
    client
        .write_u16(server_init.server_pixelformat.red_max)
        .await
        .unwrap_or(());
    client
        .write_u16(server_init.server_pixelformat.green_max)
        .await
        .unwrap_or(());
    client
        .write_u16(server_init.server_pixelformat.blue_max)
        .await
        .unwrap_or(());
    client
        .write_u8(server_init.server_pixelformat.red_shift)
        .await
        .unwrap_or(());
    client
        .write_u8(server_init.server_pixelformat.green_shift)
        .await
        .unwrap_or(());
    client
        .write_u8(server_init.server_pixelformat.blue_shift)
        .await
        .unwrap_or(());
    client
        .write(server_init.server_pixelformat.padding.as_slice())
        .await
        .unwrap_or(0);
    client
        .write_u32(server_init.name_length)
        .await
        .unwrap_or(());
    client
        .write(server_init.name_string.as_bytes())
        .await
        .unwrap_or(0);

    /* SERVER-INIT PROCESSING COMPLETE */
    init_clientserver_handshake(client, wm).await;
}

async fn init_serverinit_handshake(client: TcpStream, wm: Arc<WindowManager>) {
    match wm.as_ref() {
        #[cfg(target_os = "windows")]
        WindowManager::WIN32(win32_server) => {
            write_serverinit_message(
                client, 
                win32::get_display_struct(win32_server.monitors[0].clone()), 
                wm
            )
            .await;
        },
        #[cfg(target_os = "linux")]
        WindowManager::X11(x11_server) => {
            /* X11-DISPLAYSTRUCT API */
            write_serverinit_message(
                client,
                x11::get_display_struct(x11_server, x11_server.displays[0].clone()),
                wm,
            )
            .await;
        }
    }
}

async fn init_clientinit_handshake(mut client: TcpStream, wm: Arc<WindowManager>) {
    match client.read_u8().await.unwrap_or(0) {
        0 => {
            /* SHARED_FLAG = 0, DISCONNECT ALL OTHERS */
            /* TRY IMPLEMENTATION */
        }
        1.. => {
            /* SHARED_FLAG != 0, SHARE SCREEN WITH ALL CLIENTS */
            init_serverinit_handshake(client, wm).await;
        }
    }
}

async fn init_securityresult_handshake(
    mut client: TcpStream,
    security_type: u8,
    wm: Arc<WindowManager>,
    auth: Option<RFBAuthentication>
) {
    match security_type {
        0 | 3.. => {
            let rfb_error = create_rfb_error(String::from("Authentication Type not Supported"));
            client
                .write_u32(rfb_error.reason_length)
                .await
                .unwrap_or(());
            client
                .write(rfb_error.reason_string.as_bytes())
                .await
                .unwrap_or(0);
        }
        1 => {
            /* HANDLE AUTHENTICATION TYPE NONE */
            client.write_u32(0).await.unwrap_or(());
            init_clientinit_handshake(client, wm).await;
        }
        2 => {
            /* HANDLE VNC AUTHENTICATION, Get Password */
            let vnc_key: [u8; 8];
            match auth.unwrap() {
                RFBAuthentication::Vnc(key) => vnc_key = key.security_key
            }

            /*
                THIS IS NOT A PART OF THE RFB PROTOCOL SPECIFICATION
                VNC Authentication reverses the order of bits
                Know more at https://catonmat.net/curious-case-of-des-algorithm
            */

            let mut vnckey_le: [u8; 8] = [0; 8];
            for index in 0..vnc_key.len() {
                vnckey_le[index] = u8::from_bits(vnc_key[index].get_bits_le(), false);
            }

            /* Create DES Encryption Object */
            let des = Des::new_from_slice(&vnckey_le);
            let des = des.unwrap();

            /* Auth Challenge Key */
            let challenge = parser::security::vnc_auth_challenge();
            client.write_u128(challenge).await.unwrap();

            /* Read Encrypted Key from Client */
            let mut challenge_buf: [u8; 16] = [0; 16];
            client.read_exact(&mut challenge_buf).await.unwrap();

            /* Decrypt Client Challenge */
            let mut decrypted_challenge: Vec<GenericArray<u8, typenum::U8>> = vec![
                GenericArray::clone_from_slice(&challenge_buf[0..8]),
                GenericArray::clone_from_slice(&challenge_buf[8..16])
            ];

            /* Call Decryptor and Verify */
            des.decrypt_blocks(&mut decrypted_challenge);
            if challenge.to_be_bytes().eq(decrypted_challenge.concat().as_slice()) {
                /* Security Result Message: Ok(0) */
                client.write_u32(0).await.unwrap_or(());
                init_clientinit_handshake(client, wm).await;
            } else {
                /* Security Result Message: Failed(1) */
                client.write_u32(1).await.unwrap_or(());

                /* Failure Reason for RFB Version 3.8 */
                let rfb_error = create_rfb_error(String::from("Password is Incorrect"));
                client.write_u32(rfb_error.reason_length).await.unwrap_or(());
                client.write(rfb_error.reason_string.as_bytes()).await.unwrap_or(0);
            }
        }
    }
}

async fn init_authentication_handshake(mut client: TcpStream, wm: Arc<WindowManager>, auth: Option<RFBAuthentication>) {
    /* INITIATE SECURITY HANDSHAKE, VNC_SERVER CONSTANTS */
    let mut rfb_server = RFBServer::init();
    if auth.is_some() {
        /* Fix this in future */
        rfb_server.supported_security_types = vec![2];
        rfb_server.supported_security_types_length = 1;
    }

    /* SEND AVAILABLE SECURITY METHODS */
    client
        .write_u8(rfb_server.supported_security_types_length)
        .await
        .unwrap_or(());
    client
        .write_all(rfb_server.supported_security_types.as_slice())
        .await
        .unwrap_or(());

    /* READ CLIENT RESPONSE */
    match client.read_u8().await {
        Ok(selected_type) => init_securityresult_handshake(client, selected_type, wm, auth).await,
        Err(_) => {
            client.shutdown().await.unwrap_or(());
        }
    }
}

async fn init_handshake(mut client: TcpStream, wm: Arc<WindowManager>, auth: Option<RFBAuthentication>) {
    let rfb_server = RFBServer::init();
    let mut buf: [u8; 12] = [0; 12];
    client
        .write(&rfb_server.protocol_version)
        .await
        .unwrap_or(0);
    match client.read_exact(&mut buf).await {
        Ok(protocol_index) => {
            if &buf[0..protocol_index] == b"RFB 003.008\n" {
                debug::l1(format!("RFB Client agreed on V3.8"));
                init_authentication_handshake(client, wm, auth).await;
            } else {
                let rfb_error = create_rfb_error(String::from("Version not Supported"));
                client
                    .write_u32(rfb_error.reason_length)
                    .await
                    .unwrap_or(());
                client
                    .write(rfb_error.reason_string.as_bytes())
                    .await
                    .unwrap_or(0);
            }
        }
        Err(_) => {
            client.shutdown().await.unwrap_or(());
        }
    }
}

pub async fn create(options: CreateOptions) -> Result<(), Box<dyn Error>> {
    let tcplistener_result = TcpListener::bind(options.ip_address).await;
    if tcplistener_result.is_ok() {
        let listener = tcplistener_result.unwrap();
        let tcp_address = listener.local_addr().unwrap();
        debug::l1(format!("SpifyRFB is accepting connections on {:?}\n", tcp_address));
        
        /* Define WindowManager Object */
        let wm_arc: Option<Arc<WindowManager>>;

        if options.spify_daemon {
            /* Send IP Address Update to Daemon */
            ipc_client::send_hello(
                format!("{}\r\n{}", process::id().to_string(), tcp_address)
            ).await;
        }
        
        if options.ws_proxy.is_some() {
            /* Unwrap Proxy Parameters */
            let ws_proxy = options.ws_proxy.unwrap();
            let ws_tcp_address = ws_proxy.0;
            let ws_secure = ws_proxy.1;

            let proxy_address = listener.local_addr().unwrap().to_string();
            let proxy_address_array: Vec<&str> = proxy_address.split(":").collect();
            let proxy_address_parsed: String;

            if proxy_address_array[0] == "0.0.0.0" {
                proxy_address_parsed = format!("localhost:{}", proxy_address_array[1]);
            } else {
                proxy_address_parsed = proxy_address;
            }

            tokio::spawn(async move {
                websocket::create(
                    WSCreateOptions {
                        tcp_address: ws_tcp_address,
                        proxy_address: proxy_address_parsed,
                        secure: ws_secure,
                        spify_daemon: options.spify_daemon,
                    }
                ).await.unwrap();
            });
        }

        #[cfg(target_os = "windows")]
        {
            let win32_connection = win32::connect(options.spify_daemon);
            if win32_connection.is_ok() {
                wm_arc = Option::Some(win32_connection.unwrap());
            } else {
                /* Return Win32 Connection Error */
                return Err(String::from("Windows API Connection Error").into());
            }
        }

        #[cfg(target_os = "linux")]
        {
            let x11_connection = x11::connect();
            if x11_connection.is_ok() {
                wm_arc = Option::Some(x11_connection.unwrap());
            } else {
                /* Return X11 Connection Error */
                return Err(String::from("X11 Connection Error").into());                    
            }
        }
        
        if wm_arc.is_some() {
            /* Unwrap WindowManager Object */
            let wm_arc = wm_arc.unwrap();

            /* Accept All Incoming Connections */
            loop {
                let (client, _) = listener.accept().await?;
                let wm = Arc::clone(&wm_arc);
                let auth_clone = options.auth.clone();
    
                tokio::spawn(async move {
                    /* Init Handshake */
                    debug::l1(format!("Connection Established: {:?}", client));
                    init_handshake(client, wm, auth_clone).await;
                });
            }
        } else {
            /* WM Platform Not Supported */
            return Err(String::from("Window Manager Platform Not Supported").into());
        }           
    } else {
        let err = tcplistener_result.err().unwrap();
        debug::l1(format!("IP Address Binding Failed -> {}", err.to_string()));
        return Err(err.into());
    }
}