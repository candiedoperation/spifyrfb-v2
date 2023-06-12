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

use super::{FrameBufferRectangle, FrameBuffer, RFBEncodingType};

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

    /* Divide FrameBuffer into Tiles of 64x64 pixels */
    let h_tiles = (framebuffer.width as f32 / HEXTILE_WIDTH).ceil() as usize;
    let v_tiles = (framebuffer.height as f32 / HEXTILE_HEIGHT).ceil() as usize;

    let mut hextiles: Vec<Vec<u8>> = vec![Vec::new(); v_tiles * h_tiles];
    let hscan_lines: Vec<&[u8]>;
    hscan_lines = framebuffer
        .raw_pixels
        .chunks_exact((framebuffer.width * bytes_per_pixel) as usize)
        .collect();

    let mut vertical_tile = 0;
    let mut hscan_line_ctr = 0;
    for hscan_line in hscan_lines {
        let mut current_tile = vertical_tile * (h_tiles);
        for h_chunk in hscan_line
            .chunks((HEXTILE_WIDTH as u16 * bytes_per_pixel) as usize)
            .collect::<Vec<_>>()
        {
            hextiles[current_tile].extend_from_slice(h_chunk);
            current_tile += 1;
        }

        if hscan_line_ctr == (HEXTILE_HEIGHT as usize - 1) {
            hscan_line_ctr = 0;
            vertical_tile += 1;
        } else {
            hscan_line_ctr += 1;
        }
    }

    let mut compressed_hextiles: Vec<u8> = Vec::with_capacity(hextiles.capacity());
    let mut solid_previous_tile: (bool, Vec<u8>) = (false, Vec::with_capacity(1));

    for hextile in hextiles {
        let solid_hextile = solid_hextile_color(hextile.clone(), bytes_per_pixel as usize);
        if solid_hextile.0 == true {
            if solid_hextile.1 != solid_previous_tile.1 {
                compressed_hextiles.push(2_u8);
                compressed_hextiles.extend_from_slice(solid_hextile.1.as_slice());
            } else {
                /* Set No bits, color same as previous tile */
                compressed_hextiles.push(0_u8);
            }
        } else {
            compressed_hextiles.push(1_u8);
            compressed_hextiles.extend_from_slice(hextile.as_slice());
        }

        /* Update Previous Hextile (for Solid Color) */
        solid_previous_tile = solid_hextile.clone();
    }

    /* Send Compressed Tiles */
    compressed_hextiles
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