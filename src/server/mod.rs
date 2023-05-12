use crate::x11;
use image::EncodableLayout;
use std::error::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

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

#[derive(Debug)]
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
    pub const TRLE: i32 = 15;
    pub const ZRLE: i32 = 16;
}

pub struct FrameBufferRectangle {
    pub(crate) x_position: u16,
    pub(crate) y_position: u16,
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) encoding_type: i32,
    pub(crate) pixel_data: Vec<u8>
}

pub struct FrameBufferUpdate {
    pub(crate) message_type: u8,
    pub(crate) padding: u8,
    pub(crate) number_of_rectangles: u16,
    pub(crate) frame_buffer: Vec<FrameBufferRectangle>,
}

enum WindowManager<'a> {
    X11(&'a x11::X11Server),
    _WIN32(u8),
}

struct VNCServer {
    protocol_version: [u8; 12],
    supported_security_types_length: u8,
    supported_security_types: [u8; 2],
}

impl VNCServer {
    fn init() -> VNCServer {
        VNCServer {
            protocol_version: String::from("RFB 003.008\n").as_bytes().try_into().unwrap(),
            supported_security_types_length: 1,
            supported_security_types: [1, 2], /* SECURITY TYPE 0 IS INVALID */
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

async fn write_framebuffer_update_message(client: &mut TcpStream, frame_buffer: FrameBufferUpdate) {
    client.write_u8(frame_buffer.message_type).await.unwrap();
    client.write_u8(frame_buffer.padding).await.unwrap();
    client
        .write_u16(frame_buffer.number_of_rectangles)
        .await
        .unwrap();

    for framebuffer in frame_buffer.frame_buffer {
        client.write_u16(framebuffer.x_position).await.unwrap();
        client.write_u16(framebuffer.y_position).await.unwrap();
        client.write_u16(framebuffer.width).await.unwrap();
        client.write_u16(framebuffer.height).await.unwrap();
        client.write_i32(framebuffer.encoding_type).await.unwrap();
        client.write(framebuffer.pixel_data.as_bytes()).await.unwrap();
    }
}

async fn process_clientserver_message(
    client: &mut TcpStream,
    message: &[u8],
    wm: WindowManager<'_>,
) {
    match message[0] {
        ClientToServerMessage::SET_PIXEL_FORMAT => {
            let _pixelformat_request: &[u8] = &message[4..];
            /* SET PIXEL FORMAT IN FUTURE RELEASES */

            match wm {
                WindowManager::_WIN32(_win32_server) => {}
                WindowManager::X11(x11_server) => {
                    write_framebuffer_update_message(
                        client,
                        x11::fullscreen_framebuffer_update(&x11_server, 0, RFBEncodingType::RAW),
                    )
                    .await;
                }
            }
        }
        ClientToServerMessage::SET_ENCODINGS => {
            println!("Set Encodings Request");
        }
        ClientToServerMessage::FRAME_BUFFER_UPDATE_REQUEST => {
            /* let incremental: u8 = message[1]; */
            let x_position: u16 = ((message[2] as u16) << 8) | message[3] as u16;
            let y_position: u16 = ((message[4] as u16) << 8) | message[5] as u16;
            let width: u16 = ((message[6] as u16) << 8) | message[7] as u16;
            let height: u16 = ((message[8] as u16) << 8) | message[9] as u16;

            match wm {
                WindowManager::_WIN32(_win32_server) => {}
                WindowManager::X11(x11_server) => {
                    write_framebuffer_update_message(
                        client,
                        x11::rectangle_framebuffer_update(
                            &x11_server, 
                            0, 
                            RFBEncodingType::RAW, 
                            x_position.try_into().unwrap(),
                            y_position.try_into().unwrap(),
                            width,
                            height
                        ),
                    )
                    .await;
                }
            }
        }
        ClientToServerMessage::KEY_EVENT => {
            
        }
        ClientToServerMessage::POINTER_EVENT => { 
            
        }
        ClientToServerMessage::CLIENT_CUT_TEXT => {}
        _ => {}
    }
}

async fn init_clientserver_handshake(mut client: TcpStream) {
    /* PERSISTENT X11 CONNECTION TO PREVENT A ZILLION CONNECTIONS ON CLIENT EVENTS */
    match x11::connect() {
        Ok(x11_server) => {
            loop {
                let mut buffer: [u8; 512] = [0; 512];
                match client.read(&mut buffer[..]).await {
                    // Return value of `Ok(0)` signifies that the remote has close
                    Ok(0) => {
                        println!("Client Has Disconnected");
                        return;
                    }
                    Ok(n) => {
                        process_clientserver_message(
                            &mut client,
                            &buffer[..n],
                            WindowManager::X11(&x11_server),
                        )
                        .await
                    }
                    Err(_) => {
                        // Unexpected client error. There isn't much we can do
                        // here so just stop processing.
                        return;
                    }
                }
            }
        }
        Err(_) => {
            println!("x11-server Connection Error");
            return;
        }
    };
}

async fn write_serverinit_message(mut client: TcpStream, server_init: RFBServerInit) {
    client
        .write_u16(server_init.framebuffer_width)
        .await
        .unwrap();
    client
        .write_u16(server_init.framebuffer_height)
        .await
        .unwrap();
    client
        .write_u8(server_init.server_pixelformat.bits_per_pixel)
        .await
        .unwrap();
    client
        .write_u8(server_init.server_pixelformat.depth)
        .await
        .unwrap();
    client
        .write_u8(server_init.server_pixelformat.big_endian_flag)
        .await
        .unwrap();
    client
        .write_u8(server_init.server_pixelformat.true_color_flag)
        .await
        .unwrap();
    client
        .write_u16(server_init.server_pixelformat.red_max)
        .await
        .unwrap();
    client
        .write_u16(server_init.server_pixelformat.green_max)
        .await
        .unwrap();
    client
        .write_u16(server_init.server_pixelformat.blue_max)
        .await
        .unwrap();
    client
        .write_u8(server_init.server_pixelformat.red_shift)
        .await
        .unwrap();
    client
        .write_u8(server_init.server_pixelformat.green_shift)
        .await
        .unwrap();
    client
        .write_u8(server_init.server_pixelformat.blue_shift)
        .await
        .unwrap();
    client
        .write(server_init.server_pixelformat.padding.as_bytes())
        .await
        .unwrap();
    client.write_u32(server_init.name_length).await.unwrap();
    client
        .write(server_init.name_string.as_bytes())
        .await
        .unwrap();

    /* SERVER-INIT PROCESSING COMPLETE */
    init_clientserver_handshake(client).await;
}

async fn init_serverinit_handshake(client: TcpStream) {
    /* X11-DISPLAYSTRUCT API */
    write_serverinit_message(client, x11::get_display_struct(None, 0)).await;
}

async fn init_clientinit_handshake(mut client: TcpStream) {
    match client.read_u8().await.unwrap_or(0) {
        0 => {
            /* SHARED_FLAG = 0, DISCONNECT ALL OTHERS */
            /* TRY IMPLEMENTATION */
        }
        1.. => {
            /* SHARED_FLAG != 0, SHARE SCREEN WITH ALL CLIENTS */
            init_serverinit_handshake(client).await;
        }
    }
}

async fn init_securityresult_handshake(mut client: TcpStream, security_type: u8) {
    match security_type {
        0 | 3.. => {
            let rfb_error = create_rfb_error(String::from("Authentication Type not Supported"));
            client.write_u32(rfb_error.reason_length).await.unwrap();
            client
                .write(rfb_error.reason_string.as_bytes())
                .await
                .unwrap();
        }
        1 => {
            /* HANDLE AUTHENTICATION TYPE NONE */
            client.write_u32(0).await.unwrap();
            init_clientinit_handshake(client).await;
        }
        2 => { /* HANDLE AUTHENTICATION TYPE VNC */ }
    }
}

async fn init_authentication_handshake(mut client: TcpStream) {
    /* INITIATE SECURITY HANDSHAKE, VNC_SERVER CONSTANTS */
    let vnc_server = VNCServer::init();

    /* SEND AVAILABLE SECURITY METHODS */
    client
        .write_u8(vnc_server.supported_security_types_length)
        .await
        .unwrap();
    client
        .write_u8(vnc_server.supported_security_types[0])
        .await
        .unwrap();

    /* READ CLIENT RESPONSE */
    match client.read_u8().await {
        Ok(selected_type) => init_securityresult_handshake(client, selected_type).await,
        Err(_) => {
            client.shutdown().await.unwrap();
        }
    }
}

async fn init_handshake(mut client: TcpStream) {
    //print!("Buffer: {}", String::from_utf8_lossy(buffer));

    let vnc_server = VNCServer::init();
    let mut buf: [u8; 12] = [0; 12];
    client.write(&vnc_server.protocol_version).await.unwrap();
    match client.read_exact(&mut buf).await {
        Ok(protocol_index) => {
            if &buf[0..protocol_index] == b"RFB 003.008\n" {
                println!("RFB Client agreed on V3.8");
                init_authentication_handshake(client).await;
            } else {
                let rfb_error = create_rfb_error(String::from("Version not Supported"));
                client.write_u32(rfb_error.reason_length).await.unwrap();
                client
                    .write(rfb_error.reason_string.as_bytes())
                    .await
                    .unwrap();
            }
        }
        Err(_) => {
            client.shutdown().await.unwrap();
        }
    }
}

pub async fn create() -> Result<(), Box<dyn Error>> {
    /* Create a Tokio TCP Listener on Free Port */
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    loop {
        let (client, _) = listener.accept().await?;
        tokio::spawn(async move {
            // Handle The Client
            println!("Connection Established: {:?}", client);
            init_handshake(client).await;
        });
    }
}
