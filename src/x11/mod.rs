use std::{
    sync::Arc,
};

use crate::server::{
    self, FrameBufferRectangle, FrameBufferUpdate, PixelFormat, RFBEncodingType, RFBServerInit,
    ServerToClientMessage, WindowManager,
};
use x11rb::{
    connection::Connection,
    protocol::{
        xproto::{
            self, ImageFormat, Screen,
        }, xtest,
    },
    rust_connection::{ConnectError, RustConnection},
};

pub struct X11Server {
    pub(crate) connection: RustConnection,
    pub(crate) displays: Vec<xproto::Screen>,
}

pub struct X11PointerEvent {
    pub(crate) dst_x: i16,
    pub(crate) dst_y: i16,
    pub(crate) button_mask: u8,
}

pub fn fire_pointer_event(
    x11_server: &X11Server,
    x11_screen: Screen,
    x11_pointer_event: X11PointerEvent,
) {
    xtest::fake_input(
        &x11_server.connection, 
        xproto::MOTION_NOTIFY_EVENT, 
        false.into(), 
        x11rb::CURRENT_TIME, 
        x11_screen.root.clone(), 
        x11_pointer_event.dst_x, 
        x11_pointer_event.dst_y, 
        0
    ).unwrap();

    xtest::fake_input(
        &x11_server.connection, 
        if x11_pointer_event.button_mask == 0 { xproto::BUTTON_RELEASE_EVENT } else { xproto::BUTTON_PRESS_EVENT }, 
        if x11_pointer_event.button_mask == 0 { 3 } else { x11_pointer_event.button_mask },
        x11rb::CURRENT_TIME, 
        x11_screen.root.clone(),
        x11_pointer_event.dst_x, 
        x11_pointer_event.dst_y, 
        0
    ).unwrap();
}

pub fn get_display_struct(x11_server: &X11Server, x11_screen: Screen) -> server::RFBServerInit {
    RFBServerInit {
        framebuffer_width: x11_screen.width_in_pixels,
        framebuffer_height: x11_screen.height_in_pixels,
        server_pixelformat: PixelFormat {
            bits_per_pixel: if x11_screen.root_depth == 24 {
                32
            } else {
                x11_screen.root_depth
            }, /* ADD ALPHA-CHANNEL IF TRUE-COLOR */
            depth: x11_screen.root_depth,
            big_endian_flag: 0,
            true_color_flag: (x11_screen.root_depth == 24).into(),
            red_max: if x11_screen.root_depth == 24 {
                2_u16.pow(8) - 1
            } else {
                0
            },
            green_max: if x11_screen.root_depth == 24 {
                2_u16.pow(8) - 1
            } else {
                0
            },
            blue_max: if x11_screen.root_depth == 24 {
                2_u16.pow(8) - 1
            } else {
                0
            },
            red_shift: 0, /* COMMENT */
            green_shift: 0,
            blue_shift: 0,
            padding: [0, 0, 0],
        },
        name_length: (x11_server).connection.setup().vendor_len().into(),
        name_string: String::from_utf8(x11_server.connection.setup().clone().vendor).unwrap(),
    }
}

pub fn fullscreen_framebuffer_update(
    x11_server: &X11Server,
    x11_screen: Screen,
    encoding_type: i32,
) -> FrameBufferUpdate {
    let x11_cookie = xproto::get_image(
        &x11_server.connection,
        ImageFormat::Z_PIXMAP,
        x11_screen.root,
        0,
        0,
        x11_screen.width_in_pixels,
        x11_screen.height_in_pixels,
        !0,
    )
    .unwrap()
    .reply();

    let pixel_chunks = x11_cookie.unwrap().data;
    let pixel_chunks: Vec<&[u8]> = pixel_chunks.chunks(4).collect();
    let mut pixel_data: Vec<u8> = vec![];

    for pixel in pixel_chunks {
        pixel_data.push(pixel[0]);
        pixel_data.push(pixel[1]);
        pixel_data.push(pixel[2]);
        pixel_data.push(255);
    }

    let mut frame_buffer: Vec<FrameBufferRectangle> = vec![];
    match encoding_type {
        RFBEncodingType::RAW => {
            frame_buffer.push(FrameBufferRectangle {
                x_position: 0,
                y_position: 0,
                width: x11_screen.width_in_pixels,
                height: x11_screen.height_in_pixels,
                encoding_type: RFBEncodingType::RAW,
                pixel_data,
            });
        }
        _ => {}
    }

    FrameBufferUpdate {
        message_type: ServerToClientMessage::FRAME_BUFFER_UPDATE,
        padding: 0,
        number_of_rectangles: 1,
        frame_buffer,
    }
}

pub fn rectangle_framebuffer_update(
    x11_server: &X11Server,
    x11_screen: Screen,
    encoding_type: i32,
    x_position: i16,
    y_position: i16,
    width: u16,
    height: u16,
) -> FrameBufferUpdate {
    let x11_cookie = xproto::get_image(
        &x11_server.connection,
        ImageFormat::Z_PIXMAP,
        x11_screen.root,
        x_position,
        y_position,
        width,
        height,
        !0,
    )
    .unwrap()
    .reply();

    let pixel_chunks = x11_cookie.unwrap().data;
    let pixel_chunks: Vec<&[u8]> = pixel_chunks.chunks(4).collect();
    let mut pixel_data: Vec<u8> = vec![];

    for pixel in pixel_chunks {
        pixel_data.push(pixel[0]);
        pixel_data.push(pixel[1]);
        pixel_data.push(pixel[2]);
        pixel_data.push(255);
    }

    let mut frame_buffer: Vec<FrameBufferRectangle> = vec![];
    match encoding_type {
        RFBEncodingType::RAW => {
            frame_buffer.push(FrameBufferRectangle {
                x_position: x_position.try_into().unwrap(),
                y_position: y_position.try_into().unwrap(),
                width,
                height,
                encoding_type: RFBEncodingType::RAW,
                pixel_data,
            });
        }
        _ => {}
    }

    FrameBufferUpdate {
        message_type: ServerToClientMessage::FRAME_BUFFER_UPDATE,
        padding: 0,
        number_of_rectangles: 1,
        frame_buffer,
    }
}

pub fn connect() -> Result<Arc<WindowManager>, ConnectError> {
    match x11rb::connect(None) {
        Ok((x11_connection, _x11_screen_id)) => {
            return Ok(Arc::new(WindowManager::X11(X11Server {
                displays: x11_connection.setup().clone().roots,
                connection: x11_connection,
            })));
        }
        Err(x11_connect_error) => {
            return Err(x11_connect_error);
        }
    };
}
