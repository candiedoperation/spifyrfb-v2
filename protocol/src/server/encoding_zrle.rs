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

use crate::server::encoding_zlib::deflate;
use super::{FrameBuffer, FrameBufferRectangle};

pub fn get_pixel_data(framebuffer: FrameBuffer, stream_id: String) -> FrameBufferRectangle {
    let c_pixels: Vec<u8> = framebuffer.encoded_pixels.clone();
    let encoded_tiles: Vec<u8>;
    if framebuffer.width > 0 && framebuffer.height > 0 {
        encoded_tiles = encode(FrameBuffer {
            encoded_pixels: c_pixels,
            ..framebuffer.clone()
        });
    } else {
        encoded_tiles = framebuffer.raw_pixels.clone();
    }
    
    /* Add encoded_structure fields */
    deflate(FrameBuffer {
        encoded_pixels: encoded_tiles,
        ..framebuffer.clone()
    }, stream_id)
}

fn encode(framebuffer: FrameBuffer) -> Vec<u8> {
    let bytes_per_cpixel: u16 = if framebuffer.bits_per_pixel >= 24 { 3 } else { (framebuffer.bits_per_pixel / 8) as u16 };
    const ZRLE_TILE_WIDTH: f32 = 64_f32;
    const ZRLE_TILE_HEIGHT: f32 = 64_f32;

    /* Divide FrameBuffer into Tiles of 64x64 pixels */
    let h_tiles = (framebuffer.width as f32 / ZRLE_TILE_WIDTH).ceil() as usize;
    let v_tiles = (framebuffer.height as f32 / ZRLE_TILE_HEIGHT).ceil() as usize;

    let mut zrle_tiles: Vec<u8> = vec![];
    let hscan_lines: Vec<&[u8]>;
    hscan_lines = framebuffer.encoded_pixels.chunks_exact((framebuffer.width * bytes_per_cpixel) as usize).collect();

    for zrletile_ctr in 0..(v_tiles * h_tiles) {
        let mut previous_scan_line: Vec<u8> = vec![];
        let mut solid_tile = true;

        let mut tile_pixels: Vec<u8> = Vec::with_capacity(
            (ZRLE_TILE_WIDTH * ZRLE_TILE_HEIGHT) as usize
             * bytes_per_cpixel as usize
        );

        let vertical_progress = ((zrletile_ctr as f32 / h_tiles as f32).floor()) as usize;
        let horizontal_progress = zrletile_ctr % h_tiles;
        let start = vertical_progress * ZRLE_TILE_HEIGHT as usize;
        let end = 
            if (start + ZRLE_TILE_HEIGHT as usize) > hscan_lines.len() { hscan_lines.len() } 
            else { start + ZRLE_TILE_HEIGHT as usize };

        for hscan_line_ctr in start..end {
            let h_start = horizontal_progress * bytes_per_cpixel as usize * (ZRLE_TILE_WIDTH as usize);
            let h_end = 
            if (h_start + (ZRLE_TILE_HEIGHT as usize * bytes_per_cpixel as usize)) > hscan_lines[hscan_line_ctr].len() { hscan_lines[hscan_line_ctr].len() } 
            else { h_start + (ZRLE_TILE_HEIGHT as usize * bytes_per_cpixel as usize) };

            let scan_line = &hscan_lines[hscan_line_ctr][h_start..h_end];
            tile_pixels.extend_from_slice(scan_line);

            if previous_scan_line.len() == 0 {
                /* This is the first Line */
                previous_scan_line = scan_line.to_vec();
            } else {
                if scan_line != previous_scan_line {
                    /* This is not a Solid Tile */
                    solid_tile = false;
                }
            }
        }

        if solid_tile == true {
            let mut solid_color: Vec<u8> = vec![];
            for subpixel in 0..(bytes_per_cpixel as usize) {
                solid_color.push(tile_pixels[subpixel]);
            }

            zrle_tiles.push(1_u8);
            zrle_tiles.extend_from_slice(&solid_color);
        } else {
            zrle_tiles.push(0_u8);
            zrle_tiles.extend_from_slice(tile_pixels.as_slice());
        }
    }

    /* Send Compressed Tiles */
    zrle_tiles
}