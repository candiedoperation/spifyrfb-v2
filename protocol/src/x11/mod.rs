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

mod keycodes;
use std::{collections::HashMap, sync::Arc};
use crate::server::{
    self, FrameBufferRectangle, FrameBufferUpdate, PixelFormat, RFBEncodingType, RFBServerInit,
    ServerToClientMessage, WindowManager, encoding_raw, encoding_zrle, encoding_zlib, encoding_hextile, FrameBuffer,
};

use x11rb::{
    connection::Connection,
    protocol::{
        xproto::{self, ImageFormat, KeyButMask, Screen},
        xtest,
    },
    rust_connection::{ConnectError, RustConnection},
};

pub struct X11Server {
    pub(crate) connection: RustConnection,
    pub(crate) displays: Vec<xproto::Screen>,
    pub(crate) keysym_map: HashMap<u32, u8>,
}

pub struct X11PointerEvent {
    pub(crate) dst_x: i16,
    pub(crate) dst_y: i16,
    pub(crate) button_mask: u8,
}

pub struct X11KeyEvent {
    pub(crate) key_down: u8,
    pub(crate) key_sym: u32,
}

fn parse_keybutmask(mask: KeyButMask) -> u8 {
    match mask {
        KeyButMask::BUTTON1 => 1,
        KeyButMask::BUTTON2 => 2,
        KeyButMask::BUTTON3 => 3,
        KeyButMask::BUTTON4 => 4,
        KeyButMask::BUTTON5 => 5,
        _ => 1,
    }
}

pub fn fire_key_event(x11_server: &X11Server, x11_screen: Screen, x11_keyevent: X11KeyEvent) {
    keycodes::create_keysym_map(&x11_server.connection);
    xtest::fake_input(
        &x11_server.connection,
        if x11_keyevent.key_down == 0 {
            xproto::KEY_RELEASE_EVENT
        } else {
            xproto::KEY_PRESS_EVENT
        },
        *x11_server.keysym_map.get(&x11_keyevent.key_sym).unwrap_or(&0),
        x11rb::CURRENT_TIME,
        x11_screen.root,
        0,
        0,
        0,
    )
    .unwrap();
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
        0,
    )
    .unwrap();

    /*
        https://manpages.ubuntu.com/manpages/bionic/man3/X11::Protocol::Ext::XTEST.3pm.html
        Be careful when faking a "ButtonPress" as it might be important to fake a matching
        "ButtonRelease" too.  On the X.org server circa 1.9.x after a synthetic press the
        physical mouse doesn't work to generate a release and the button is left hung
        (presumably in its normal implicit pointer grab).
    */

    let query_pointer_cookie = xproto::query_pointer(&x11_server.connection, x11_screen.root);
    let query_pointer_cookie = query_pointer_cookie.unwrap().reply().unwrap();

    xtest::fake_input(
        &x11_server.connection,
        if x11_pointer_event.button_mask == 0 {
            xproto::BUTTON_RELEASE_EVENT
        } else {
            xproto::BUTTON_PRESS_EVENT
        },
        if x11_pointer_event.button_mask == 0 {
            parse_keybutmask(query_pointer_cookie.mask)
        } else {
            x11_pointer_event.button_mask
        },
        x11rb::CURRENT_TIME,
        x11_screen.root,
        x11_pointer_event.dst_x,
        x11_pointer_event.dst_y,
        0,
    )
    .unwrap();
}

pub fn get_pixelformat(x11_screen: Screen) -> server::PixelFormat {
    PixelFormat {
        bits_per_pixel: if x11_screen.root_depth == 24 {
            32
        } else {
            x11_screen.root_depth
        }, /* ADD ALPHA-CHANNEL IF TRUE-COLOR */
        depth: x11_screen.root_depth,
        big_endian_flag: 1,
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
    }
}

pub fn get_display_struct(x11_server: &X11Server, x11_screen: Screen) -> server::RFBServerInit {
    RFBServerInit {
        framebuffer_width: x11_screen.width_in_pixels,
        framebuffer_height: x11_screen.height_in_pixels,
        server_pixelformat: get_pixelformat(x11_screen),
        name_length: (x11_server).connection.setup().vendor_len().into(),
        name_string: String::from_utf8(x11_server.connection.setup().clone().vendor).unwrap(),
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
    pixelformat: PixelFormat,
    zstream_id: String
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

    let mut pixel_data = x11_cookie.unwrap().data;
    let bits_per_pixel = if x11_screen.root_depth == 24 { 32 } else { x11_screen.root_depth };
    
    /* Define Shifts */
    let red = (pixelformat.red_shift / 8) as usize;
    let green = (pixelformat.green_shift / 8) as usize;
    let blue = (pixelformat.blue_shift / 8) as usize;

    let mut pixformat_data: Vec<u8> = Vec::with_capacity(pixel_data.len());
    let pixel_chunks: Vec<&mut [u8]> = pixel_data.chunks_mut((bits_per_pixel / 8) as usize).collect();

    for pixel in pixel_chunks {
        let pixel_copy = pixel.to_owned();
        pixel[red] = pixel_copy[2];
        pixel[green] = pixel_copy[1];
        pixel[blue] = pixel_copy[0];
        
        /* X11 Sets RGBA (Alpha) to Padding */
        pixel[3] = 255;
        
        if encoding_type == RFBEncodingType::ZRLE {
            /* Extend Encoded Data for ZRLE */
            pixformat_data.extend_from_slice(&[
                pixel[0],
                pixel[1],
                pixel[2]
            ]);
        }
    }

    let mut framebuffer_rectangles: Vec<FrameBufferRectangle> = vec![];
    let mut framebuffer_struct = FrameBuffer {
        x_position: x_position as u16,
        y_position: y_position as u16,
        width,
        height,
        bits_per_pixel,
        raw_pixels: pixel_data.clone(),
        encoding: RFBEncodingType::RAW,
        encoded_pixels: vec![],
    };

    match encoding_type {
        RFBEncodingType::RAW => {
            framebuffer_struct.encoding = RFBEncodingType::RAW;
            framebuffer_rectangles.push(encoding_raw::get_pixel_data(framebuffer_struct));
        },
        RFBEncodingType::ZRLE => {
            framebuffer_struct.encoding = RFBEncodingType::ZRLE;
            framebuffer_struct.encoded_pixels = pixformat_data;
            framebuffer_rectangles.push(encoding_zrle::get_pixel_data(framebuffer_struct, zstream_id));
        },
        RFBEncodingType::ZLIB => {
            framebuffer_struct.encoding = RFBEncodingType::ZLIB;
            framebuffer_rectangles.push(encoding_zlib::get_pixel_data(framebuffer_struct, zstream_id));
        },
        RFBEncodingType::HEX_TILE => {
            framebuffer_struct.encoding = RFBEncodingType::HEX_TILE;
            framebuffer_rectangles.push(encoding_hextile::get_pixel_data(framebuffer_struct));
        }
        _ => {}
    }

    FrameBufferUpdate {
        message_type: ServerToClientMessage::FRAME_BUFFER_UPDATE,
        padding: 0,
        number_of_rectangles: 1,
        frame_buffer: framebuffer_rectangles,
    }
}

pub fn connect() -> Result<Arc<WindowManager>, ConnectError> {
    match x11rb::connect(None) {
        Ok((x11_connection, _x11_screen_id)) => {
            return Ok(Arc::new(WindowManager::X11(X11Server {
                displays: x11_connection.setup().clone().roots,
                keysym_map: keycodes::create_keysym_map(&x11_connection),
                connection: x11_connection,
            })));
        }
        Err(x11_connect_error) => {
            return Err(x11_connect_error);
        }
    };
}
