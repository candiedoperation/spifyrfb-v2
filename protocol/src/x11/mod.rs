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
    ServerToClientMessage, WindowManager,
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
        *x11_server.keysym_map.get(&x11_keyevent.key_sym).unwrap(),
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
                keysym_map: keycodes::create_keysym_map(&x11_connection),
                connection: x11_connection,
            })));
        }
        Err(x11_connect_error) => {
            return Err(x11_connect_error);
        }
    };
}
