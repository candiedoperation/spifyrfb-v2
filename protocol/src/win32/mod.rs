/*
    SpifyRFB - Modern RFB Server implementation using Rust
    Copyright (C) 2023  Atheesh Thirumalairajan

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

mod keycodes;
use std::collections::HashMap;
use std::mem;
use std::sync::Arc;
use windows::Win32::Foundation::BOOL;
use windows::Win32::Graphics::Gdi as Win32_Gdi;
use windows::Win32::Foundation as Win32_Foundation;
use windows::Win32::Networking::WinSock as Win32_WinSock;
use windows::Win32::UI::WindowsAndMessaging as Win32_WindowsAndMessaging;
use windows::Win32::UI::Input::KeyboardAndMouse as Win32_KeyboardAndMouse;

use crate::server;
use crate::server::FrameBufferRectangle;
use crate::server::FrameBufferUpdate;
use crate::server::PixelFormat;
use crate::server::RFBEncodingType;
use crate::server::RFBServerInit;
use crate::server::ServerToClientMessage;
use crate::server::WindowManager;
use crate::server::encoding_raw;
use crate::server::encoding_zrle;
use crate::server::encoding_zrle::ZRLE;

trait ToU16Vec {
    fn to_u16_vec(input: String) -> Vec<u16>;
}

impl ToU16Vec for String {
    fn to_u16_vec(input: String) -> Vec<u16> {
        let mut string_utf16 = input.encode_utf16().collect::<Vec<_>>();
        string_utf16.push(0);
        string_utf16
    }
}

#[derive(Clone, Debug)]
pub struct Win32Monitor {
    _monitor_handle: Win32_Gdi::HMONITOR,
    pub monitor_rect: Win32_Foundation::RECT,
    normalized_x: i32,
    normalized_y: i32
}

struct Win32CaptureDriver {
    desktop_dc: Win32_Gdi::HDC,
    compatible_dc: Win32_Gdi::CreatedHDC,
}

pub struct Win32Server {
    pub(crate) monitors: Vec<Win32Monitor>,
    capture_driver: Win32CaptureDriver,
    keysym_vk_map: HashMap<u32, Win32_KeyboardAndMouse::VIRTUAL_KEY>
}

pub struct Win32PointerEvent {
    pub(crate) dst_x: i16,
    pub(crate) dst_y: i16,
    pub(crate) button_mask: u8,
}

pub fn fire_key_event(
    win32_server: &Win32Server,
    keysym: u32,
    down_flag: u8
) {
    unsafe {
        let mut inputs_array: [Win32_KeyboardAndMouse::INPUT; 1] = [
            Win32_KeyboardAndMouse::INPUT {
                r#type: Win32_KeyboardAndMouse::INPUT_KEYBOARD,
                Anonymous:  Win32_KeyboardAndMouse::INPUT_0 {
                    ki: Win32_KeyboardAndMouse::KEYBDINPUT { 
                        wVk: *win32_server.keysym_vk_map.get(&keysym).unwrap_or(&Win32_KeyboardAndMouse::VIRTUAL_KEY(0)), 
                        time: 0, 
                        ..Default::default()
                    }
                }
            }
        ];

        /* SEND KEYBOARD INPUT */
        if down_flag == 0 { inputs_array[0].Anonymous.ki.dwFlags = Win32_KeyboardAndMouse::KEYEVENTF_KEYUP }
        Win32_KeyboardAndMouse::SendInput(&inputs_array, mem::size_of::<Win32_KeyboardAndMouse::INPUT>() as i32);
    }
}

pub fn fire_pointer_event(
    pointer_event: Win32PointerEvent,
    input_monitor: Win32Monitor
) {
    unsafe {
        /*
            RFB BUTTON MASKS (Observed):
                BUTTON_UP:     0b00000000 = 0d0
                BUTTON_LEFT:   0b00000001 = 0d1
                BUTTON_MIDDLE: 0b00000010 = 0d2
                BUTTON_RIGHT:  0b00000100 = 0d4
                BTN_SCROLLUP:  0b00001000 = 0d8
                BTN_SCROLLDN:  0b00010000 = 0d16
        */

        let input_dwflags: Win32_KeyboardAndMouse::MOUSE_EVENT_FLAGS = match pointer_event.button_mask {
            0 => {
                if Win32_KeyboardAndMouse::GetKeyState(Win32_KeyboardAndMouse::VK_LBUTTON.0 as i32) < 0 {
                    Win32_KeyboardAndMouse::MOUSEEVENTF_MOVE | Win32_KeyboardAndMouse::MOUSEEVENTF_LEFTUP
                } else if Win32_KeyboardAndMouse::GetKeyState(Win32_KeyboardAndMouse::VK_MBUTTON.0 as i32) < 0 {
                    Win32_KeyboardAndMouse::MOUSEEVENTF_MOVE | Win32_KeyboardAndMouse::MOUSEEVENTF_MIDDLEUP
                } else if Win32_KeyboardAndMouse::GetKeyState(Win32_KeyboardAndMouse::VK_RBUTTON.0 as i32) < 0 {
                    Win32_KeyboardAndMouse::MOUSEEVENTF_MOVE | Win32_KeyboardAndMouse::MOUSEEVENTF_RIGHTUP
                } else {
                    Win32_KeyboardAndMouse::MOUSEEVENTF_MOVE
                }
            },
            1 => Win32_KeyboardAndMouse::MOUSEEVENTF_MOVE | Win32_KeyboardAndMouse::MOUSEEVENTF_LEFTDOWN,
            2 => Win32_KeyboardAndMouse::MOUSEEVENTF_MOVE | Win32_KeyboardAndMouse::MOUSEEVENTF_MIDDLEDOWN,
            4 => Win32_KeyboardAndMouse::MOUSEEVENTF_MOVE | Win32_KeyboardAndMouse::MOUSEEVENTF_RIGHTDOWN,
            8 => Win32_KeyboardAndMouse::MOUSEEVENTF_WHEEL,
            16 => Win32_KeyboardAndMouse::MOUSEEVENTF_WHEEL,
            _ => Win32_KeyboardAndMouse::MOUSEEVENTF_MOVE
        };

        let mut input_mousedata: i32 = 0;
        if pointer_event.button_mask == 8 {
            input_mousedata = Win32_WindowsAndMessaging::WHEEL_DELTA as i32
        } else if pointer_event.button_mask == 16 {
            input_mousedata = -(Win32_WindowsAndMessaging::WHEEL_DELTA as i32)
        }

        let inputs_array: [Win32_KeyboardAndMouse::INPUT; 1] = [
            Win32_KeyboardAndMouse::INPUT {
                r#type: Win32_KeyboardAndMouse::INPUT_MOUSE,
                Anonymous:  Win32_KeyboardAndMouse::INPUT_0 {
                    mi: Win32_KeyboardAndMouse::MOUSEINPUT { 
                        dx: pointer_event.dst_x as i32 * input_monitor.normalized_x, 
                        dy: pointer_event.dst_y as i32 * input_monitor.normalized_y, 
                        mouseData: input_mousedata,
                        dwFlags: Win32_KeyboardAndMouse::MOUSEEVENTF_ABSOLUTE | input_dwflags, 
                        time: 0, 
                        ..Default::default()
                    }
                }
            }
        ];

        /* SEND WARP+ACTION INPUTS */
        Win32_KeyboardAndMouse::SendInput(&inputs_array, mem::size_of::<Win32_KeyboardAndMouse::INPUT>() as i32);
    }
}

pub fn rectangle_framebuffer_update(
    win32_server: &Win32Server,
    _win32_monitor: Win32Monitor, 
    encoding_type: i32,
    x_position: i16,
    y_position: i16,
    width: u16,
    height: u16
) -> FrameBufferUpdate {
    unsafe {
        let compatible_bitmap = Win32_Gdi::CreateCompatibleBitmap(win32_server.capture_driver.desktop_dc, width as i32, height as i32);
        let compatible_dc = win32_server.capture_driver.compatible_dc;
        Win32_Gdi::SelectObject(compatible_dc, compatible_bitmap);
        Win32_Gdi::StretchBlt(
            compatible_dc,
            x_position as i32, 
            y_position as i32, 
            width as i32, 
            height as i32, 
            win32_server.capture_driver.desktop_dc, 
            x_position as i32, 
            y_position  as i32,
            width as i32, 
            height as i32, 
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
                ..Default::default()
                
            },
            ..Default::default()
        };
        
        let mut pixel_data: Vec<u8> = vec![0; (4 * width as usize * height as usize) as usize];
        Win32_Gdi::GetDIBits(
            compatible_dc,
            compatible_bitmap,
            0,
            height as u32, 
            Some(pixel_data.as_mut_ptr() as *mut core::ffi::c_void),
            &mut bitmap_info, 
            Win32_Gdi::DIB_RGB_COLORS
        );

        /* DESTROY BITMAP AFTER SAVE, DEALLOC OBJECTS ON CLOSE */
        Win32_Gdi::DeleteObject(compatible_bitmap);

        let mut frame_buffer: Vec<FrameBufferRectangle> = vec![];
        match encoding_type {
            RFBEncodingType::RAW => {
                frame_buffer.push(FrameBufferRectangle {
                    x_position: 0,
                    y_position: 0,
                    width,
                    height,
                    encoding_type: RFBEncodingType::RAW,
                    pixel_data: encoding_raw::get_pixel_data(pixel_data)
                });
            },
            RFBEncodingType::ZRLE => {
                frame_buffer.push(FrameBufferRectangle { 
                    x_position: 0, 
                    y_position: 0, 
                    width, 
                    height, 
                    encoding_type: RFBEncodingType::ZRLE, 
                    pixel_data: encoding_zrle::get_pixel_data(ZRLE {
                        width,
                        height,
                        bytes_per_pixel: 32,
                        framebuffer: pixel_data,
                    })
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
                        _monitor_handle: monitor_handle, 
                        monitor_rect: monitor_info.rcMonitor,
                        normalized_x: 65535 / (monitor_info.rcMonitor.right - monitor_info.rcMonitor.left),
                        normalized_y: 65535 / (monitor_info.rcMonitor.bottom - monitor_info.rcMonitor.top)
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
                let desktop_device_context = Win32_Gdi::GetDC(Win32_Foundation::HWND::default());
                let dest_device_context = Win32_Gdi::CreateCompatibleDC(desktop_device_context);
                let keysym_vk_map = keycodes::create_keysym_vk_map();

                return Ok(Arc::from(WindowManager::WIN32(Win32Server {
                    monitors: WIN32_MONITORS.to_vec(),
                    keysym_vk_map,
                    capture_driver: Win32CaptureDriver { 
                        desktop_dc: desktop_device_context, 
                        compatible_dc: dest_device_context
                    }
                })));
            },
            _ => {
                return Err(String::from("Win32API MonitorFetch Error"))
            }
        }
    }
}