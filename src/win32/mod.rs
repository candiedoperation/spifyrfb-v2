use std::ffi;
use std::mem;
use std::sync::Arc;
use windows::Win32::Foundation::BOOL;
use windows::Win32::Graphics::Gdi as Win32_Gdi;
use windows::Win32::Foundation as Win32_Foundation;
use windows::Win32::Networking::WinSock as Win32_WinSock;

use crate::server;
use crate::server::FrameBufferRectangle;
use crate::server::FrameBufferUpdate;
use crate::server::PixelFormat;
use crate::server::RFBEncodingType;
use crate::server::RFBServerInit;
use crate::server::ServerToClientMessage;
use crate::server::WindowManager;

#[derive(Clone, Debug)]
pub struct Win32Monitor {
    monitor_handle: Win32_Gdi::HMONITOR,
    pub monitor_rect: Win32_Foundation::RECT,
}

struct Win32CaptureDriver {
    desktop_dc: Win32_Gdi::HDC,
    destination_dc: Win32_Gdi::CreatedHDC,
}

pub struct Win32Server {
    pub(crate) monitors: Vec<Win32Monitor>,
    capture_driver: Win32CaptureDriver
}

pub fn rectangle_framebuffer_update(
    win32_server: &Win32Server,
    win32_monitor: Win32Monitor, 
    encoding_type: i32,
    x_position: i16,
    y_position: i16,
    width: u16,
    height: u16
) -> FrameBufferUpdate {
    unsafe {
        let compatible_bitmap = Win32_Gdi::CreateCompatibleBitmap(win32_server.capture_driver.desktop_dc, width as i32, height as i32);
        Win32_Gdi::SelectObject(win32_server.capture_driver.desktop_dc, compatible_bitmap);
        Win32_Gdi::BitBlt(
            win32_server.capture_driver.destination_dc,
            x_position as i32, 
            y_position as i32, 
            width as i32, 
            height as i32, 
            Option::None, 
            x_position as i32, 
            y_position  as i32, 
            Win32_Gdi::SRCCOPY
        );

        let mut bitmap_info = Win32_Gdi::BITMAPINFO {
            bmiHeader:  Win32_Gdi::BITMAPINFOHEADER { 
                biSize: mem::size_of::<Win32_Gdi::BITMAPINFOHEADER>() as u32, 
                biWidth: width as i32, 
                biHeight: height as i32 * -1,
                biPlanes: 1, /* MUST BE SET TO ONE */ 
                biBitCount: 32,
                biCompression: 0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
                
            },
            bmiColors: [Win32_Gdi::RGBQUAD {
                rgbBlue: 255,
                rgbGreen: 255,
                rgbRed: 255,
                rgbReserved: 0,
            }; 1]
        };
        
        let mut buf: Vec<u8> = Vec::with_capacity(width as usize * height as usize * 4);
        let rv = Win32_Gdi::GetDIBits(
            win32_server.capture_driver.desktop_dc, 
            compatible_bitmap,
            0,
            height as u32, 
            Some(buf.as_mut_ptr().cast()),
            &mut bitmap_info, 
            Win32_Gdi::DIB_RGB_COLORS
        );

        /* DESTROY BITMAP AFTER SAVE */
        println!("BMP: {:?} -> {:?}", rv, buf);
        Win32_Gdi::DeleteObject(compatible_bitmap);

        let mut pixel_data: Vec<u8> = vec![];
        let mut frame_buffer: Vec<FrameBufferRectangle> = vec![];
        match encoding_type {
            RFBEncodingType::RAW => {
                frame_buffer.push(FrameBufferRectangle {
                    x_position: 0,
                    y_position: 0,
                    width,
                    height,
                    encoding_type: RFBEncodingType::RAW,
                    pixel_data: buf,
                });
            }
            _ => {}
        }

        FrameBufferUpdate {
            message_type: ServerToClientMessage::FRAME_BUFFER_UPDATE,
            padding: 0,
            number_of_rectangles: 1,
            frame_buffer,
        }
    }
}

pub fn get_display_struct(win32_monitor: Win32Monitor) -> server::RFBServerInit {
    unsafe {
        let mut hostname: [u16; 15] = [0; 15];
        Win32_WinSock::GetHostNameW(&mut hostname);
        let valid_hostname = hostname.iter().position(|&c| c as u8 == b'\0' ).unwrap_or(hostname.len());
        let valid_hostname: String = String::from_utf16_lossy(&hostname[0..valid_hostname]);

        /*
            Note: Apps that you design to target Windows 8 and later can no longer 
            query or set display modes that are less than 32 bits per pixel (bpp); 
            these operations will fail. These apps have a compatibility manifest that 
            targets Windows 8. Windows 8 still supports 8-bit and 16-bit color modes 
            for desktop apps that were built without a Windows 8 manifest; Windows 8 
            emulates these modes but still runs in 32-bit color mode.
        */

        let pixel_format = PixelFormat {
            bits_per_pixel: 32,
            depth: 24, /* WINDOWS EMULATES FOR TRUE-COLOR */
            big_endian_flag: 0,
            true_color_flag: 1,
            red_max: 2_u16.pow(8) - 1,
            green_max: 2_u16.pow(8) - 1,
            blue_max: 2_u16.pow(8) - 1,
            red_shift: 0,
            green_shift: 0,
            blue_shift: 0,
            padding: [0, 0, 0]
        };
    
        RFBServerInit {
            framebuffer_width: (win32_monitor.monitor_rect.right - win32_monitor.monitor_rect.left) as u16,
            framebuffer_height: (win32_monitor.monitor_rect.bottom - win32_monitor.monitor_rect.top) as u16,
            server_pixelformat: pixel_format,
            name_length: valid_hostname.len() as u32,
            name_string: valid_hostname
        }
    }
}

pub fn connect() -> Result<Arc<WindowManager>, String> {
    unsafe {
        static mut WIN32_MONITORS: Vec<Win32Monitor> = vec![];
        unsafe extern "system" fn display_monitors(monitor_handle: Win32_Gdi::HMONITOR, _device_context: Win32_Gdi::HDC, _bound_rect: *mut Win32_Foundation::RECT,_app_data: Win32_Foundation::LPARAM) -> BOOL {
            let mut monitor_info: Win32_Gdi::MONITORINFO = Win32_Gdi::MONITORINFO::default();
            monitor_info.cbSize = mem::size_of::<Win32_Gdi::MONITORINFO>() as u32;
            let monitor_info_ptr = &mut monitor_info as *mut _;
            let get_monitor_result = Win32_Gdi::GetMonitorInfoW(monitor_handle, monitor_info_ptr);

            /* RETURN BOOL FOR CALLBACK */
            match get_monitor_result {
                Win32_Foundation::TRUE => {
                    WIN32_MONITORS.push(Win32Monitor { 
                        monitor_handle, 
                        monitor_rect: monitor_info.rcMonitor
                    });

                    /* RETURN TRUE TO FFI CALLER */
                    Win32_Foundation::TRUE
                },
                _ => {
                    Win32_Foundation::FALSE
                },
            }
        }

        let monitor_enum_proc: Win32_Gdi::MONITORENUMPROC = Option::Some(display_monitors);        
        let enum_display_monitors_result = Win32_Gdi::EnumDisplayMonitors(
            Option::None,
            Option::None,
            monitor_enum_proc, 
            Win32_Foundation::LPARAM(0)
        );

        match enum_display_monitors_result {
            Win32_Foundation::TRUE => {
                let desktop_device_context = Win32_Gdi::GetDC(Option::None);
                let dest_device_context = Win32_Gdi::CreateCompatibleDC(desktop_device_context);
        
                return Ok(Arc::from(WindowManager::WIN32(Win32Server {
                    monitors: WIN32_MONITORS.to_vec(),
                    capture_driver: Win32CaptureDriver { 
                        desktop_dc: desktop_device_context, 
                        destination_dc: dest_device_context
                    }
                })));
            },
            _ => {
                return Err(String::from("Win32API MonitorFetch Error"))
            }
        }
    }
}