use super::{encoding_zlib::deflate, encoding_raw::RawPixelData};

pub fn get_pixel_data(pixel_data: Vec<u8>) -> RawPixelData {
    let mut t_pixels: Vec<u8> = vec![];
    for pixel in pixel_data.chunks(4).collect::<Vec<&[u8]>>() {
        /* CPIXELS are only three bytes */
        t_pixels.push(pixel[0]);
        t_pixels.push(pixel[1]);
        t_pixels.push(pixel[2]);
    }

    //Define Compression Control Byte, Uses CopyFilter
    let mut encoded_data: Vec<u8> = vec![];
    let compression_control: u8 = 0b00000110;
    let compression_method: u8 = 0;
    let zlib_pixel_data = deflate(t_pixels);

    encoded_data.push(compression_control);
    encoded_data.push(compression_method);
    encoded_data.extend_from_slice(&zlib_pixel_data.pixel_data_len.to_be_bytes()[1..]);
    encoded_data.extend_from_slice(&zlib_pixel_data.pixel_data);
    RawPixelData { pixel_data: encoded_data }
}