pub struct TRLE {
    width: u16,
    height: u16,
    bytes_per_pixel: u8,
    framebuffer: Vec<u8>
}

pub fn encode(pixel_data: TRLE) -> Vec<u8> {
    /* Divide FrameBuffer into Tiles of 16x16 pixels */
    let h_tiles = (pixel_data.width as f32 / 16_f32).ceil() as usize;
    let v_tiles = (pixel_data.height as f32 / 16_f32).ceil() as usize;

    let mut trle_tiles: Vec<Vec<u8>> = Vec::with_capacity(v_tiles * h_tiles);
    let hscan_lines: Vec<&[u8]>;
    hscan_lines = pixel_data.framebuffer.chunks_exact((pixel_data.width * pixel_data.bytes_per_pixel as u16) as usize).collect();

    let mut vertical_tile = 0;
    let mut hscan_line_ctr = 0;
    for hscan_line in hscan_lines {
        let mut current_tile = vertical_tile * 15;
        for h_chunk in hscan_line.chunks((16 * pixel_data.bytes_per_pixel) as usize).collect::<Vec<_>>() {
            trle_tiles[current_tile].extend_from_slice(h_chunk);
            current_tile += 1;
        }

        if hscan_line_ctr == 5 {
            hscan_line_ctr = 0;
            vertical_tile += 1;
        } else {
            hscan_line_ctr += 1;
        }
    }

    vec![]
}