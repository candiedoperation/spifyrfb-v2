use flate2::{Compression, FlushCompress, Compress};

pub struct ZRLE {
    pub width: u16,
    pub height: u16,
    pub bytes_per_pixel: u8,
    pub framebuffer: Vec<u8>
}

pub fn get_pixel_data(pixel_data: ZRLE) -> Vec<u8> {
    let mut c_pixels: Vec<u8> = vec![];
    for pixel in pixel_data.framebuffer.chunks(4).collect::<Vec<&[u8]>>() {
        /* CPIXELS are only three bytes */
        c_pixels.push(pixel[0]);
        c_pixels.push(pixel[1]);
        c_pixels.push(pixel[2]);
    }

    let mut encoded_structure: Vec<u8> = vec![];
    let encoded_pixels = encode(ZRLE {
        width: if pixel_data.width == 0 { 1 } else { pixel_data.width },
        height: if pixel_data.height == 0 { 1 } else { pixel_data.height },
        bytes_per_pixel: pixel_data.bytes_per_pixel,
        framebuffer: c_pixels,
    });
    
    /* Add encoded_structure fields */
    encoded_structure.extend_from_slice(&(encoded_pixels.len() as u32).to_be_bytes());
    encoded_structure.extend_from_slice(encoded_pixels.as_slice());
    encoded_structure
}

fn encode(pixel_data: ZRLE) -> Vec<u8> {
    let bytes_per_cpixel: u16 = if pixel_data.bytes_per_pixel == 32 { 3 } else { pixel_data.bytes_per_pixel as u16 };
    const ZRLE_TILE_WIDTH: f32 = 64_f32;
    const ZRLE_TILE_HEIGHT: f32 = 64_f32;

    /* Divide FrameBuffer into Tiles of 64x64 pixels */
    let h_tiles = (pixel_data.width as f32 / ZRLE_TILE_WIDTH).ceil() as usize;
    let v_tiles = (pixel_data.height as f32 / ZRLE_TILE_HEIGHT).ceil() as usize;

    let mut zrle_tiles: Vec<Vec<u8>> = vec![Vec::new(); v_tiles * h_tiles];
    let hscan_lines: Vec<&[u8]>;
    hscan_lines = pixel_data.framebuffer.chunks_exact((pixel_data.width * bytes_per_cpixel) as usize).collect();

    let mut vertical_tile = 0;
    let mut hscan_line_ctr = 0;
    for hscan_line in hscan_lines {
        let mut current_tile = vertical_tile * (h_tiles - 1);
        for h_chunk in hscan_line.chunks((64 * bytes_per_cpixel) as usize).collect::<Vec<_>>() {
            if hscan_line_ctr == 0 { zrle_tiles[current_tile].push(0_u8); }
            zrle_tiles[current_tile].extend_from_slice(h_chunk);
            current_tile += 1;
        }

        if hscan_line_ctr == 63 {
            hscan_line_ctr = 0;
            vertical_tile += 1;
        } else {
            hscan_line_ctr += 1;
        }
    }

    let flattened_tiles= zrle_tiles.into_iter().flatten().collect::<Vec<u8>>();
    let mut compressed_tiles: Vec<u8> = Vec::with_capacity(flattened_tiles.len());

    let mut compressor = Compress::new(Compression::new(6), true);
    compressor.compress_vec(
        flattened_tiles.as_slice(), 
        &mut compressed_tiles,
        FlushCompress::None
    ).unwrap();

    /* Send Compressed Tiles */
    compressed_tiles
}