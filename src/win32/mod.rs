use std::mem;
use std::sync::Arc;
use windows::Win32::Foundation::BOOL;
use windows::Win32::Graphics::Direct3D9 as Win32_D3D9;
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
    monitor_rect: Win32_Foundation::RECT,
    _monitor_work: Win32_Foundation::RECT
}

pub struct Win32Server {
    pub(crate) monitors: Vec<Win32Monitor>
}

pub fn fullscreen_framebuffer_update(win32_monitor: Win32Monitor, encoding_type: i32) -> FrameBufferUpdate {
    let mut pixel_data: Vec<u8> = vec![];
    
    let mut frame_buffer: Vec<FrameBufferRectangle> = vec![];
    match encoding_type {
        RFBEncodingType::RAW => {
            frame_buffer.push(FrameBufferRectangle {
                x_position: 0,
                y_position: 0,
                width: (win32_monitor.monitor_rect.right - win32_monitor.monitor_rect.left) as u16,
                height: (win32_monitor.monitor_rect.bottom - win32_monitor.monitor_rect.top) as u16,
                encoding_type: RFBEncodingType::RAW,
                pixel_data,
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

pub fn get_display_struct(win32_monitor: Win32Monitor) -> server::RFBServerInit {
    unsafe {
        let mut hostname: [u16; 15] = [0; 15];
        Win32_WinSock::GetHostNameW(&mut hostname);

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
            name_length: 15,
            name_string: String::from_utf16_lossy(&hostname)
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
                        monitor_rect: monitor_info.rcMonitor, 
                        _monitor_work: monitor_info.rcWork 
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
                return Ok(Arc::from(WindowManager::WIN32(Win32Server {
                    monitors: WIN32_MONITORS.to_vec()
                })));
            },
            _ => {
                return Err(String::from("Win32API MonitorFetch Error"))
            }
        }
    }
}