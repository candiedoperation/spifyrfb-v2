use super::{encoding_zlib::deflate, encoding_raw::RawPixelData};

#[derive(Debug)]
pub struct TightPixelData {
    pub compression_control: u8,
    pub compression_method: u8,
    pub pixel_data_len: Vec<u8>,
    pub pixel_data: Vec<u8>
}

pub fn get_pixel_data(pixel_data: Vec<u8>) -> TightPixelData {
    let mut t_pixels: Vec<u8> = vec![];
    for pixel in pixel_data.chunks(4).collect::<Vec<&[u8]>>() {
        /* CPIXELS are only three bytes */
        t_pixels.push(pixel[0]);
        t_pixels.push(pixel[1]);
        t_pixels.push(pixel[2]);
    }

    //Define Compression Control Byte, Uses CopyFilter
    let compression_control: u8 = 0b10001010;
    let compression_method: u8 = 0;
    let zlib_pixel_data = deflate(t_pixels);

    TightPixelData {
        compression_control,
        compression_method,
        pixel_data_len: (zlib_pixel_data.pixel_data_len.to_be_bytes()[1..]).to_vec(),
        pixel_data: zlib_pixel_data.pixel_data,
    }
}