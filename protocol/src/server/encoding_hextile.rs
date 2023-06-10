pub struct Hextile {
    pub width: u16,
    pub height: u16,
    pub bits_per_pixel: u8,
    pub framebuffer: Vec<u8>,
}

pub fn get_pixel_data(pixel_data: Hextile) -> Vec<u8> {
    encode(Hextile {
        width: if pixel_data.width == 0 { 1 } else { pixel_data.width },
        height: if pixel_data.height == 0 { 1 } else { pixel_data.height },
        bits_per_pixel: pixel_data.bits_per_pixel,
        framebuffer: pixel_data.framebuffer,
    })
}

fn encode(pixel_data: Hextile) -> Vec<u8> {
    let bytes_per_pixel: u16 = 4;
    const HEXTILE_WIDTH: f32 = 16_f32;
    const HEXTILE_HEIGHT: f32 = 16_f32;

    /* Divide FrameBuffer into Tiles of 64x64 pixels */
    let h_tiles = (pixel_data.width as f32 / HEXTILE_WIDTH).ceil() as usize;
    let v_tiles = (pixel_data.height as f32 / HEXTILE_HEIGHT).ceil() as usize;

    let mut hextiles: Vec<Vec<u8>> = vec![Vec::new(); v_tiles * h_tiles];
    let hscan_lines: Vec<&[u8]>;
    hscan_lines = pixel_data
        .framebuffer
        .chunks_exact((pixel_data.width * bytes_per_pixel) as usize)
        .collect();

    let mut vertical_tile = 0;
    let mut hscan_line_ctr = 0;
    for hscan_line in hscan_lines {
        let mut current_tile = vertical_tile * (h_tiles);
        for h_chunk in hscan_line
            .chunks((HEXTILE_WIDTH as u16 * bytes_per_pixel) as usize)
            .collect::<Vec<_>>()
        {
            //if hscan_line_ctr == 0 { hextiles[current_tile].push(1_u8); }
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
        let solid_hextile = solid_hextile_color(hextile.clone());
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

fn solid_hextile_color(tile: Vec<u8>) -> (bool, Vec<u8>) {
    let tile_chunks: Vec<&[u8]> = tile.chunks(4).collect();
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