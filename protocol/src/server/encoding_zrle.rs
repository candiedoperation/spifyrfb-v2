use crate::server::encoding_zlib::deflate;
use super::encoding_zlib::ZlibPixelData;

#[derive(Clone)]
pub struct ZRLE {
    pub width: u16,
    pub height: u16,
    pub bytes_per_pixel: u8,
    pub framebuffer: Vec<u8>,
    pub session: String
}

pub fn get_pixel_data(pixel_data: ZRLE) -> ZlibPixelData {
    let mut c_pixels: Vec<u8> = vec![];
    for pixel in pixel_data.framebuffer.chunks(4).collect::<Vec<&[u8]>>() {
        /* CPIXELS are only three bytes */
        c_pixels.push(pixel[0]);
        c_pixels.push(pixel[1]);
        c_pixels.push(pixel[2]);
    }

    let encoded_tiles: Vec<u8>;
    if pixel_data.width > 0 && pixel_data.height > 0 {
        encoded_tiles = encode(ZRLE {
            framebuffer: c_pixels,
            ..pixel_data.clone()
        });
    } else {
        encoded_tiles = Vec::with_capacity(1);
    }
    
    /* Add encoded_structure fields */
    deflate(encoded_tiles, pixel_data.session)
}

fn encode(pixel_data: ZRLE) -> Vec<u8> {
    let bytes_per_cpixel: u16 = 3;
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
        let mut current_tile = vertical_tile * (h_tiles);
        for h_chunk in hscan_line.chunks((ZRLE_TILE_WIDTH as u16 * bytes_per_cpixel) as usize).collect::<Vec<_>>() {
            zrle_tiles[current_tile].extend_from_slice(h_chunk);
            current_tile += 1;
        }

        if hscan_line_ctr == (ZRLE_TILE_HEIGHT as usize - 1) {
            hscan_line_ctr = 0;
            vertical_tile += 1;
        } else {
            hscan_line_ctr += 1;
        }
    }

    let mut compressed_zrletiles: Vec<u8> = Vec::with_capacity(zrle_tiles.capacity());
    for zrle_tile in zrle_tiles {
        let solid_zrletile = solid_zrletile_color(zrle_tile.clone());
        if solid_zrletile.0 == true {
            compressed_zrletiles.push(1_u8);
            compressed_zrletiles.extend_from_slice(solid_zrletile.1.as_slice());
        } else {
            compressed_zrletiles.push(0_u8);
            compressed_zrletiles.extend_from_slice(zrle_tile.as_slice());
        }
    }

    /* Send Compressed Tiles */
    compressed_zrletiles
}

fn solid_zrletile_color(tile: Vec<u8>) -> (bool, Vec<u8>) {
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