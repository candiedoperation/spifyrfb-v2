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

pub fn get_pixel_data(framebuffer: FrameBuffer, session: String) -> FrameBufferRectangle {
    let mut c_pixels: Vec<u8> = vec![];
    for pixel in framebuffer.raw_pixels.chunks(4).collect::<Vec<&[u8]>>() {
        /* CPIXELS are only three bytes */
        c_pixels.push(pixel[0]);
        c_pixels.push(pixel[1]);
        c_pixels.push(pixel[2]);
    }

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
    }, session)
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

            tile_pixels.extend_from_slice(&hscan_lines[hscan_line_ctr][
                h_start..h_end
            ]);
        }

        let solid_zrletile = solid_zrletile_color(tile_pixels.clone(), bytes_per_cpixel as usize);
        if solid_zrletile.0 == true {
            zrle_tiles.push(1_u8);
            zrle_tiles.extend_from_slice(solid_zrletile.1.as_slice());
        } else {
            zrle_tiles.push(0_u8);
            zrle_tiles.extend_from_slice(tile_pixels.as_slice());
        }
    }

    /* Send Compressed Tiles */
    zrle_tiles
}

fn solid_zrletile_color(tile: Vec<u8>, bytes_per_pixel: usize) -> (bool, Vec<u8>) {
    let tile_chunks: Vec<&[u8]> = tile.chunks(bytes_per_pixel).collect();
    let initial_color = tile_chunks[0];
    let mut solid_color: bool = true;

    for tile_chunk in tile_chunks {
        if tile_chunk != initial_color {
            solid_color = false;
            break;
        }
    }

    if solid_color == true {
        (true, initial_color.to_vec())
    } else {
        (false, Vec::with_capacity(1))
    }
}