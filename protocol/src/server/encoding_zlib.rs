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

use std::{mem, ptr};
use super::{session, FrameBufferRectangle, FrameBuffer, RFBEncodingType};

pub fn create_zlib_stream() -> libz_sys::z_stream {
    libz_sys::z_stream {
        next_in: ptr::null_mut(),
        avail_in: 0,
        total_in: 0,
        next_out: ptr::null_mut(),
        avail_out: 0,
        total_out: 0,
        msg: ptr::null::<u8>() as _,
        state: ptr::null::<u8>() as _,
        zalloc: unsafe { mem::transmute(ptr::null::<u8>()) },
        zfree: unsafe { mem::transmute(ptr::null::<u8>()) },
        opaque: ptr::null::<u8>() as _,
        data_type: libz_sys::Z_BINARY,
        adler: 0,
        reserved: 0,
    }
}

pub fn deflate(framebuffer: FrameBuffer, session: String) -> FrameBufferRectangle {
    let zlib_data = framebuffer.encoded_pixels;
    let max_compressed = zlib_data.len() + ((zlib_data.len() + 99) / 100) + 12;
    let mut next_in: Vec<u8> = zlib_data.clone();
    let mut next_out: Vec<u8> = vec![0; max_compressed];

    let mut framebuffer_rectangle = FrameBufferRectangle {
        x_position: framebuffer.x_position,
        y_position: framebuffer.y_position,
        width: framebuffer.width,
        height: framebuffer.height,
        encoding_type: RFBEncodingType::RAW,
        encoded_pixels: framebuffer.raw_pixels,
        encoded_pixels_length: 0,
    };

    unsafe {
        let mut zlib_stream = session::get_zlib_stream(session.clone());
        zlib_stream.next_in = next_in.as_mut_ptr();
        zlib_stream.avail_in = next_in.len() as u32;
        zlib_stream.next_out = next_out.as_mut_ptr();
        zlib_stream.avail_out = max_compressed as u32;

        if zlib_stream.total_in == 0 {
            /* Init ZLIB Stream */
            println!("Initializing Zlib Stream");

            /* Call deflateInit2_ */
            let deflate_init_status = libz_sys::deflateInit2_(
                &mut zlib_stream,
                2, /* Set Compress Level (0-9, None-Max) */
                libz_sys::Z_DEFLATED,
                15, /* Range: 8-15 (Min-Max Memory) */
                8,
                libz_sys::Z_DEFAULT_STRATEGY,
                libz_sys::zlibVersion(),
                mem::size_of::<libz_sys::z_stream>() as i32,
            );

            if deflate_init_status != libz_sys::Z_OK {
                println!("ZLIB: DeflateInit2_() failed (RAW Sent). Status: {}", deflate_init_status);
                return framebuffer_rectangle;
            }
        }

        let previous_total_out = zlib_stream.total_out;
        let deflate_status = libz_sys::deflate(
            &mut zlib_stream,
            libz_sys::Z_SYNC_FLUSH
        );

        if deflate_status != libz_sys::Z_OK {
            println!("ZLIB: Deflate() failed (RAW Sent). Status: {}", deflate_status);
            return framebuffer_rectangle;
        }

        /* Calculate Compression and Update Stream */
        let compressed_bytes = zlib_stream.total_out - previous_total_out;
        session::update_zlib_stream(session, zlib_stream);

        /* Update FrameBufferRectangle */
        framebuffer_rectangle.encoded_pixels_length = compressed_bytes as u32;
        framebuffer_rectangle.encoding_type = framebuffer.encoding;
        framebuffer_rectangle.encoded_pixels = next_out[..(compressed_bytes as usize)].to_vec();
        framebuffer_rectangle
    }
}

pub fn get_pixel_data(framebuffer: FrameBuffer, session: String) -> FrameBufferRectangle {
    deflate(framebuffer, session)
}
