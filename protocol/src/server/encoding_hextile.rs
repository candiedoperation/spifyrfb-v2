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

use super::{FrameBuffer, FrameBufferRectangle, RFBEncodingType};

pub fn get_pixel_data(framebuffer: FrameBuffer) -> FrameBufferRectangle {
    let mut framebuffer_rectangle = FrameBufferRectangle {
        x_position: framebuffer.x_position,
        y_position: framebuffer.y_position,
        width: framebuffer.width,
        height: framebuffer.height,
        encoding_type: RFBEncodingType::RAW,
        encoded_pixels: framebuffer.clone().raw_pixels,
        encoded_pixels_length: 0,
    };

    if framebuffer.width > 0 && framebuffer.height > 0 {
        /* Update FrameBufferRectangle */
        framebuffer_rectangle.encoding_type = RFBEncodingType::HEX_TILE;
        framebuffer_rectangle.encoded_pixels = encode(framebuffer);
        framebuffer_rectangle
    } else {
        /* Send RAW Format */
        framebuffer_rectangle
    }
}

fn encode(framebuffer: FrameBuffer) -> Vec<u8> {
    let bytes_per_pixel: u16 = (framebuffer.bits_per_pixel / 8) as u16;
    const HEXTILE_WIDTH: f32 = 16_f32;
    const HEXTILE_HEIGHT: f32 = 16_f32;

    /* Divide FrameBuffer into Tiles of 16x16 pixels */
    let h_tiles = (framebuffer.width as f32 / HEXTILE_WIDTH).ceil() as usize;
    let v_tiles = (framebuffer.height as f32 / HEXTILE_HEIGHT).ceil() as usize;

    let mut hextiles: Vec<u8> = vec![];
    let hscan_lines: Vec<&[u8]>;
    hscan_lines = framebuffer
        .raw_pixels
        .chunks_exact((framebuffer.width * bytes_per_pixel) as usize)
        .collect();

    let mut solid_previous_tile: (bool, Vec<u8>) = (false, Vec::with_capacity(1));
    for hextile_ctr in 0..(v_tiles * h_tiles) {
        let mut tile_pixels: Vec<u8> = Vec::with_capacity(
            (HEXTILE_WIDTH * HEXTILE_HEIGHT) as usize 
            * bytes_per_pixel as usize,
        );

        let vertical_progress = ((hextile_ctr as f32 / h_tiles as f32).floor()) as usize;
        let horizontal_progress = hextile_ctr % h_tiles;
        let start = vertical_progress * HEXTILE_HEIGHT as usize;
        let end = 
            if (start + HEXTILE_HEIGHT as usize) > hscan_lines.len() { hscan_lines.len() } 
            else { start + HEXTILE_HEIGHT as usize };

        for hscan_line_ctr in start..end {
            let h_start = horizontal_progress * bytes_per_pixel as usize * (HEXTILE_WIDTH as usize);
            let h_end = 
                if (h_start + (HEXTILE_HEIGHT as usize * bytes_per_pixel as usize)) > hscan_lines[hscan_line_ctr].len() { hscan_lines[hscan_line_ctr].len() } 
                else { h_start + (HEXTILE_HEIGHT as usize * bytes_per_pixel as usize) };

            tile_pixels.extend_from_slice(&hscan_lines[hscan_line_ctr][h_start..h_end]);
        }

        let solid_hextile = solid_hextile_color(tile_pixels.clone(), bytes_per_pixel as usize);
        if solid_hextile.0 == true {
            if solid_hextile.1 != solid_previous_tile.1 {
                hextiles.push(2_u8);
                hextiles.extend_from_slice(solid_hextile.1.as_slice());
            } else {
                /* Set No bits, color same as previous tile */
                hextiles.push(0_u8);
            }
        } else {
            hextiles.push(1_u8);
            hextiles.extend_from_slice(tile_pixels.as_slice());
        }

        /* Update Previous Hextile (for Solid Color) */
        solid_previous_tile = solid_hextile.clone();
    }

    /* Send Hextiles */
    hextiles
}

fn solid_hextile_color(tile: Vec<u8>, bytes_per_pixel: usize) -> (bool, Vec<u8>) {
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
