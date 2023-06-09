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
use windows::core as Win32_Core;
use windows::Win32::Foundation::BOOL;
use windows::Win32::Graphics::Gdi as Win32_Gdi;
use windows::Win32::Security as Win32_Security;
use windows::Win32::Foundation as Win32_Foundation;
use windows::Win32::System::Shutdown as Win32_Shutdown;
use windows::Win32::Networking::WinSock as Win32_WinSock;
use windows::Win32::System::Threading as Win32_Threading;
use windows::Win32::UI::WindowsAndMessaging as Win32_WindowsAndMessaging;
use windows::Win32::UI::Input::KeyboardAndMouse as Win32_KeyboardAndMouse;
use windows::Win32::System::StationsAndDesktops as Win32_StationsAndDesktops;

use crate::server;
use crate::server::FrameBuffer;
use crate::server::FrameBufferRectangle;
use crate::server::FrameBufferUpdate;
use crate::server::PixelFormat;
use crate::server::RFBEncodingType;
use crate::server::RFBServerInit;
use crate::server::ServerToClientMessage;
use crate::server::WindowManager;
use crate::server::encoding_hextile;
use crate::server::encoding_raw;
use crate::server::encoding_zlib;
use crate::server::encoding_zrle;

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

#[derive(Clone)]
pub struct Win32Monitor {
    _monitor_handle: Win32_Gdi::HMONITOR,
    pub monitor_rect: Win32_Foundation::RECT,
    pub monitor_devmode: Win32_Gdi::DEVMODEW,
    normalized_x: i32,
    normalized_y: i32
}

pub struct Win32Server {
    pub(crate) monitors: Vec<Win32Monitor>,
    keysym_vk_map: HashMap<u32, Win32_KeyboardAndMouse::VIRTUAL_KEY>,
    spify_daemon: bool
}

pub struct Win32PointerEvent {
    pub(crate) dst_x: i16,
    pub(crate) dst_y: i16,
    pub(crate) button_mask: u8,
}

/*
    Note: Apps that you design to target Windows 8 and later can no longer 
    query or set display modes that are less than 32 bits per pixel (bpp); 
    these operations will fail. These apps have a compatibility manifest that 
    targets Windows 8. Windows 8 still supports 8-bit and 16-bit color modes 
    for desktop apps that were built without a Windows 8 manifest; Windows 8 
    emulates these modes but still runs in 32-bit color mode.
*/

/* Define BPP Constant */
const WIN32_BITS_PER_PIXEL: u8 = 32;

pub fn lock_workstation() -> bool {
    unsafe {
        Win32_Shutdown::LockWorkStation().as_bool()
    }
}

pub fn logoff() -> bool {
    unsafe {
        Win32_Shutdown::ExitWindowsEx(
            Win32_Shutdown::EXIT_WINDOWS_FLAGS(Win32_Shutdown::EWX_LOGOFF.0 | Win32_WindowsAndMessaging::EWX_FORCEIFHUNG), 
            Win32_Shutdown::SHTDN_REASON_MINOR_TERMSRV
        ).as_bool()
    }
}

pub fn shutdown() -> bool {
    unsafe {
        get_exitwindows_priviledge();
        Win32_Shutdown::ExitWindowsEx(
            Win32_Shutdown::EXIT_WINDOWS_FLAGS(Win32_Shutdown::EWX_SHUTDOWN.0 | Win32_WindowsAndMessaging::EWX_FORCEIFHUNG), 
            Win32_Shutdown::SHTDN_REASON_MINOR_TERMSRV
        ).as_bool()
    }
}

pub fn restart() -> bool {
    unsafe {
        get_exitwindows_priviledge();
        Win32_Shutdown::ExitWindowsEx(
            Win32_Shutdown::EXIT_WINDOWS_FLAGS(Win32_Shutdown::EWX_REBOOT.0 | Win32_WindowsAndMessaging::EWX_FORCEIFHUNG), 
            Win32_Shutdown::SHTDN_REASON_MINOR_TERMSRV
        ).as_bool()
    }
}

fn get_exitwindows_priviledge() {
    unsafe {
        /* Enable SE_SHUTDOWN_NAME Priviledge, If disabled */
        let mut token_handle= Win32_Foundation::HANDLE::default();
        let mut token_luid = Win32_Foundation::LUID::default();
        Win32_Threading::OpenProcessToken(
            Win32_Threading::GetCurrentProcess(), 
            Win32_Security::TOKEN_ADJUST_PRIVILEGES, 
            &mut token_handle
        );

        /* Get SE_SHUTDOWN_NAME Priv Val */
        Win32_Security::LookupPrivilegeValueW(
            Option::None, 
            Win32_Security::SE_SHUTDOWN_NAME, 
            &mut token_luid
        );

        /* Define Token Privileges */
        let token_privileges = Win32_Security::TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [Win32_Security::LUID_AND_ATTRIBUTES {
                Luid: token_luid,
                Attributes: Win32_Security::SE_PRIVILEGE_ENABLED,
            }],
        };

        Win32_Security::AdjustTokenPrivileges(
            token_handle, 
            Win32_Foundation::FALSE, 
            Option::Some(&token_privileges), 
            0, 
            Option::None, 
            Option::None
        );        
    }
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
    height: u16,
    pixelformat: PixelFormat,
    zstream_id: String
) -> FrameBufferUpdate {
    unsafe {
        /* Get Desktop User is currently seeing */
        if win32_server.spify_daemon == true {
            let input_desktop = Win32_StationsAndDesktops::OpenInputDesktop(
                Win32_StationsAndDesktops::DESKTOP_CONTROL_FLAGS(0), /* Prevent Processes in Other Accounts to set Hooks */ 
                Win32_Foundation::FALSE, /* Processes Spawn Don't Inherit */
                Win32_StationsAndDesktops::DESKTOP_ACCESS_FLAGS(Win32_Foundation::GENERIC_ALL.0)
            );
    
            if input_desktop.is_ok() {
                let input_desktop = input_desktop.unwrap();
                Win32_StationsAndDesktops::SetThreadDesktop(input_desktop);
            }
        }

        /* Initiate Screen Capture */
        let desktop_dc = Win32_Gdi::GetDC(Option::None);
        let compatible_dc = Win32_Gdi::CreateCompatibleDC(desktop_dc);
        let compatible_bitmap = Win32_Gdi::CreateCompatibleBitmap(desktop_dc, width as i32, height as i32);
        Win32_Gdi::SelectObject(compatible_dc, compatible_bitmap);
        Win32_Gdi::StretchBlt(
            compatible_dc,
            x_position as i32, 
            y_position as i32, 
            width as i32, 
            height as i32, 
            desktop_dc, 
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
                biBitCount: WIN32_BITS_PER_PIXEL as u16,
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
        Win32_Gdi::DeleteDC(compatible_dc);
        Win32_Gdi::ReleaseDC(Win32_Foundation::HWND(0), desktop_dc);

        /* Define Shifts */
        let red = (pixelformat.red_shift / 8) as usize;
        let green = (pixelformat.green_shift / 8) as usize;
        let blue = (pixelformat.blue_shift / 8) as usize;

        /* Encode pixels according to PixelFormat */
        let mut pixformat_data: Vec<u8> = Vec::with_capacity(pixel_data.len());
        let pixel_chunks: Vec<&mut [u8]> = pixel_data.chunks_mut((WIN32_BITS_PER_PIXEL / 8) as usize).collect();

        for pixel in pixel_chunks {            
            let pixel_copy = pixel.to_owned();
            pixel[red] = pixel_copy[2];
            pixel[green] = pixel_copy[1];
            pixel[blue] = pixel_copy[0];
            
            if encoding_type == RFBEncodingType::ZRLE {
                /* Extend Encoded Data for ZRLE */
                pixformat_data.extend_from_slice(&[
                    pixel[0],
                    pixel[1],
                    pixel[2]
                ]);
            }
        }

        let mut framebuffer_rectangles: Vec<FrameBufferRectangle> = vec![];
        let mut framebuffer_struct = FrameBuffer {
            x_position: x_position as u16,
            y_position: y_position as u16,
            width,
            height,
            bits_per_pixel: WIN32_BITS_PER_PIXEL,
            raw_pixels: pixel_data,
            encoding: RFBEncodingType::RAW,
            encoded_pixels: vec![],
        };

        match encoding_type {
            RFBEncodingType::RAW => {
                framebuffer_struct.encoding = RFBEncodingType::RAW;
                framebuffer_rectangles.push(encoding_raw::get_pixel_data(framebuffer_struct));
            },
            RFBEncodingType::ZRLE => {
                framebuffer_struct.encoding = RFBEncodingType::ZRLE;
                framebuffer_struct.encoded_pixels = pixformat_data;
                framebuffer_rectangles.push(encoding_zrle::get_pixel_data(framebuffer_struct, zstream_id));
            },
            RFBEncodingType::ZLIB => {
                framebuffer_struct.encoding = RFBEncodingType::ZLIB;
                framebuffer_rectangles.push(encoding_zlib::get_pixel_data(framebuffer_struct, zstream_id));
            },
            RFBEncodingType::HEX_TILE => {
                framebuffer_struct.encoding = RFBEncodingType::HEX_TILE;
                framebuffer_rectangles.push(encoding_hextile::get_pixel_data(framebuffer_struct));
            }
            _ => {}
        }

        FrameBufferUpdate {
            message_type: ServerToClientMessage::FRAME_BUFFER_UPDATE,
            padding: 0,
            number_of_rectangles: 1,
            frame_buffer: framebuffer_rectangles,
        }
    }
}

pub fn get_pixelformat() -> server::PixelFormat {
    PixelFormat {
        bits_per_pixel: WIN32_BITS_PER_PIXEL,
        depth: 24, /* WINDOWS EMULATES FOR TRUE-COLOR */
        big_endian_flag: 1,
        true_color_flag: 1,
        red_max: 2_u16.pow(8) - 1,
        green_max: 2_u16.pow(8) - 1,
        blue_max: 2_u16.pow(8) - 1,
        red_shift: 16,
        green_shift: 8,
        blue_shift: 0,
        padding: [0, 0, 0]
    }
}

pub fn get_display_struct(win32_monitor: Win32Monitor) -> server::RFBServerInit {
    unsafe {
        let mut hostname: [u16; 15] = [0; 15];
        Win32_WinSock::GetHostNameW(&mut hostname);
        let valid_hostname = hostname.iter().position(|&c| c as u8 == b'\0' ).unwrap_or(hostname.len());
        let valid_hostname: String = String::from_utf16_lossy(&hostname[0..valid_hostname]);
    
        RFBServerInit {
            framebuffer_width: win32_monitor.monitor_devmode.dmPelsWidth as u16,
            framebuffer_height: win32_monitor.monitor_devmode.dmPelsHeight as u16,
            server_pixelformat: get_pixelformat(),
            name_length: valid_hostname.len() as u32,
            name_string: valid_hostname
        }
    }
}

pub fn connect(spify_daemon: bool) -> Result<Arc<WindowManager>, String> {
    unsafe {
        static mut WIN32_MONITORS: Vec<Win32Monitor> = vec![];
        unsafe extern "system" fn display_monitors(monitor_handle: Win32_Gdi::HMONITOR, _device_context: Win32_Gdi::HDC, _bound_rect: *mut Win32_Foundation::RECT,_app_data: Win32_Foundation::LPARAM) -> BOOL {
            let mut monitorinfoex: Win32_Gdi::MONITORINFOEXW = Win32_Gdi::MONITORINFOEXW::default();
            monitorinfoex.monitorInfo.cbSize = mem::size_of::<Win32_Gdi::MONITORINFOEXW>() as u32;
            let monitorinfoex_ptr = mem::transmute(&mut monitorinfoex);
            let get_monitor_result = Win32_Gdi::GetMonitorInfoW(monitor_handle, monitorinfoex_ptr);

            /* RETURN BOOL FOR CALLBACK */
            match get_monitor_result {
                Win32_Foundation::TRUE => {
                    /* Enumerate Display Settings to get Real Resolution */
                    let mut display_devmode: Win32_Gdi::DEVMODEW = Win32_Gdi::DEVMODEW::default();
                    display_devmode.dmSize = mem::size_of::<Win32_Gdi::DEVMODEW>() as u16;
                    let enumdisplaysettings_result = Win32_Gdi::EnumDisplaySettingsW(
                        Win32_Core::PCWSTR::from_raw(monitorinfoex.szDevice.as_ptr()), 
                        Win32_Gdi::ENUM_CURRENT_SETTINGS, 
                        &mut display_devmode
                    );

                    match enumdisplaysettings_result {
                        Win32_Foundation::TRUE => {
                            let monitor_info = monitorinfoex.monitorInfo;
                            WIN32_MONITORS.push(Win32Monitor { 
                                _monitor_handle: monitor_handle, 
                                monitor_rect: monitor_info.rcMonitor,
                                monitor_devmode: display_devmode,
                                normalized_x: 65535 / display_devmode.dmPelsWidth as i32,
                                normalized_y: 65535 / display_devmode.dmPelsHeight as i32
                            });
        
                            /* RETURN TRUE TO FFI CALLER */
                            Win32_Foundation::TRUE
                        },
                        _ => {
                            Win32_Foundation::FALSE
                        }
                    }
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
                /* Create Keysym_VK_Map */
                let keysym_vk_map = keycodes::create_keysym_vk_map();
                return Ok(Arc::from(WindowManager::WIN32(Win32Server {
                    monitors: WIN32_MONITORS.to_vec(),
                    keysym_vk_map,
                    spify_daemon
                })));
            },
            _ => {
                return Err(String::from("Win32API MonitorFetch Error"))
            }
        }
    }
}
