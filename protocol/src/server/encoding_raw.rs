use super::{FrameBuffer, FrameBufferRectangle, RFBEncodingType};

pub fn get_pixel_data(framebuffer: FrameBuffer) -> FrameBufferRectangle {
    FrameBufferRectangle {
        x_position: framebuffer.x_position,
        y_position: framebuffer.x_position,
        width: framebuffer.width,
        height: framebuffer.height,
        encoding_type: RFBEncodingType::RAW,
        encoded_pixels: framebuffer.raw_pixels,
        encoded_pixels_length: 0,
    }
}