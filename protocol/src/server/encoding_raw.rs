#[derive(Debug)]
pub struct RawPixelData {
    pub pixel_data: Vec<u8>
}

pub fn get_pixel_data(pixel_data: Vec<u8>) -> RawPixelData {
    RawPixelData { pixel_data }
}