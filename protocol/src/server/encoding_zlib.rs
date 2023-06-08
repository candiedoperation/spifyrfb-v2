use flate2::{Compression, Compress, FlushCompress};

pub fn get_pixel_data(pixel_data: Vec<u8>) -> Vec<u8> {
    let mut encoded_structure: Vec<u8> = vec![];
    let mut encoded_pixels: Vec<u8> = Vec::with_capacity(pixel_data.len());

    //let mut zlib_encoder: ZlibEncoder<Vec<u8>> = ZlibEncoder::new(Vec::new(), Compression::fast());
    //zlib_encoder.write_all(pixel_data.as_slice()).unwrap();
    //encoded_pixels = zlib_encoder.finish().unwrap();

    let mut compressor = Compress::new(Compression::new(6), true);
    compressor.compress_vec(
        pixel_data.as_slice(), 
        &mut encoded_pixels,
        FlushCompress::None
    ).unwrap();

    encoded_structure.extend_from_slice(&(encoded_pixels.len() as u32).to_be_bytes());
    encoded_structure.extend_from_slice(encoded_pixels.as_slice());
    encoded_structure
}