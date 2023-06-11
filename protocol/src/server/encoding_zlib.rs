use std::{mem, ptr};
use super::session;

#[derive(Debug)]
pub struct ZlibPixelData {
    pub pixel_data_len: u32,
    pub pixel_data: Vec<u8>
}

pub fn create_zlib_stream() -> libz_sys::z_stream {
    libz_sys::z_stream {
        next_in: ptr::null_mut(),
        avail_in: 0,
        total_in: 0,
        next_out: ptr::null_mut(),
        avail_out: 0,
        total_out: 0,
        msg: ptr::null::<u8>() as _,
        state: ptr::null::<u8>() as _,
        zalloc: unsafe { mem::transmute(ptr::null::<u8>()) },
        zfree: unsafe { mem::transmute(ptr::null::<u8>()) },
        opaque: ptr::null::<u8>() as _,
        data_type: libz_sys::Z_BINARY,
        adler: 0,
        reserved: 0,
    }
}

pub fn deflate(pixel_data: Vec<u8>, session: String) -> ZlibPixelData {
    let max_compressed = pixel_data.len() + ((pixel_data.len() + 99) / 100) + 12;
    let mut next_in: Vec<u8> = pixel_data.clone();
    let mut next_out: Vec<u8> = vec![0; max_compressed];

    unsafe {
        let mut zlib_stream = session::get_zlib_stream(session.clone());
        zlib_stream.next_in = next_in.as_mut_ptr();
        zlib_stream.avail_in = next_in.len() as u32;
        zlib_stream.next_out = next_out.as_mut_ptr();
        zlib_stream.avail_out = max_compressed as u32;

        if zlib_stream.total_in == 0 {
            /* Init ZLIB Stream */
            println!("Initializing Zlib Stream");

            /* Call deflateInit2_ */
            let deflate_init_status = libz_sys::deflateInit2_(
                &mut zlib_stream,
                5, /* Set Compress Level 6 (0-9, None-Max) */
                libz_sys::Z_DEFLATED,
                15, /* Range: 8-15 (Min-Max Memory) */
                8,
                libz_sys::Z_DEFAULT_STRATEGY,
                libz_sys::zlibVersion(),
                mem::size_of::<libz_sys::z_stream>() as i32,
            );

            if deflate_init_status != libz_sys::Z_OK {
                println!("ZLIB: DeflateInit2_() failed. Status: {}", deflate_init_status);
                return ZlibPixelData { 
                    pixel_data_len: pixel_data.len() as u32, 
                    pixel_data
                };
            }
        }

        let previous_total_out = zlib_stream.total_out;
        let deflate_status = libz_sys::deflate(
            &mut zlib_stream,
            libz_sys::Z_SYNC_FLUSH
        );

        if deflate_status != libz_sys::Z_OK {
            println!("ZLIB: Deflate() failed. Status: {}", deflate_status);
            return ZlibPixelData { 
                pixel_data_len: pixel_data.len() as u32, 
                pixel_data
            };
        }

        /* Calculate Compression and Update Stream */
        let compressed_bytes = zlib_stream.total_out - previous_total_out;
        session::update_zlib_stream(session, zlib_stream);

        ZlibPixelData { 
            pixel_data_len: compressed_bytes as u32, 
            pixel_data: (&next_out[..compressed_bytes as usize]).to_vec()
        }
    }
}

pub fn get_pixel_data(pixel_data: Vec<u8>, session: String) -> ZlibPixelData {
    deflate(pixel_data, session)
}
