use miniz_oxide::deflate;

pub fn get_pixel_data(pixel_data: Vec<u8>) -> Vec<u8> {
    let mut encoded_structure: Vec<u8> = vec![];
    let compressed_pixels = deflate::compress_to_vec_zlib(pixel_data.as_slice(), 6);

    /* Form Structure */
    encoded_structure.extend_from_slice(&(compressed_pixels.len() as u32).to_be_bytes());
    encoded_structure.extend_from_slice(compressed_pixels.as_slice());
    encoded_structure
}