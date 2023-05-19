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

use std::ffi::c_void;
use std::fs;
use std::ptr;

use windows::core as Win32_Core;
use windows::Win32::Foundation as Win32_Foundation;
use windows::Win32::System::Services as Win32_Services;
use windows::Win32::System::Threading as Win32_Threading;
use windows::Win32::System::RemoteDesktop as Win32_RemoteDesktop;

struct SpifyRFBService;
impl SpifyRFBService {
    const SERVICE_NAME: &str = "SpifyRFB Controller";
    const SERVICE_TYPE: Win32_Services::ENUM_SERVICE_TYPE = Win32_Services::SERVICE_WIN32_OWN_PROCESS;
    const SERVICE_CONTROLS: u32 = Win32_Services::SERVICE_ACCEPT_NETBINDCHANGE | Win32_Services::SERVICE_ACCEPT_STOP | Win32_Services::SERVICE_ACCEPT_SESSIONCHANGE;
}

static mut SERVICE_HANDLER: Win32_Services::SERVICE_STATUS_HANDLE = Win32_Services::SERVICE_STATUS_HANDLE(0);
pub fn create() {
    unsafe {
        let service_start_table = Win32_Services::SERVICE_TABLE_ENTRYW {
            lpServiceName: Win32_Core::PWSTR(SpifyRFBService::SERVICE_NAME.to_owned().as_mut_ptr().cast()),
            lpServiceProc: Some(start)
        };

        /* DISPATCH THE SERVICE */
        Win32_Services::StartServiceCtrlDispatcherW(&service_start_table);
    }
}

unsafe extern "system" fn start(_args_count: u32, _args_vector: *mut Win32_Core::PWSTR) {
    let service_handler_result = Win32_Services::RegisterServiceCtrlHandlerExW(
        Win32_Core::PCWSTR(SpifyRFBService::SERVICE_NAME.to_owned().as_mut_ptr().cast()), 
        Some(event_handler),
        Option::None
    );

    match service_handler_result {
        Ok(service_handler) => { 
            SERVICE_HANDLER = service_handler;
            let service_status = Win32_Services::SERVICE_STATUS {
                dwServiceType: SpifyRFBService::SERVICE_TYPE,
                dwControlsAccepted: SpifyRFBService::SERVICE_CONTROLS,
                dwCheckPoint: 0,
                dwCurrentState: Win32_Services::SERVICE_RUNNING,
                dwWin32ExitCode: Win32_Foundation::NO_ERROR.0,
                dwServiceSpecificExitCode: 0,
                ..Default::default()
            };

            /* UPDATE FUTURE STATUS HANDLER AND RUNNING STATUS WITH Win32 SERVICES */
            Win32_Services::SetServiceStatus(SERVICE_HANDLER, &service_status);

            /* FUTURE FUNCTION EXECUTION */
            start_app();
        },
        Err(_) => {
            /* RETURN SERVICE FAILURE */
            return;
        }
    } 
}

unsafe extern "system" fn event_handler(_control: u32, _control_event: u32, _control_data: *mut c_void, _control_context: *mut c_void) -> u32 {
    match _control {
        Win32_Services::SERVICE_CONTROL_STOP => {
            let service_status = Win32_Services::SERVICE_STATUS {
                dwServiceType: SpifyRFBService::SERVICE_TYPE,
                dwControlsAccepted: SpifyRFBService::SERVICE_CONTROLS,
                dwCheckPoint: 0,
                dwCurrentState: Win32_Services::SERVICE_STOPPED,
                dwWin32ExitCode: Win32_Foundation::NO_ERROR.0,
                dwServiceSpecificExitCode: 0,
                ..Default::default()
            };

            /* UPDATE FUTURE STATUS HANDLER AND RUNNING STATUS WITH Win32 SERVICES */
            Win32_Services::SetServiceStatus(SERVICE_HANDLER, &service_status);
            Win32_Foundation::NO_ERROR.0
        }
        _ => {
            /* DO NOTHING IF EVENT IS NOT RECOGNIZED */
            Win32_Foundation::NO_ERROR.0
        }
    }
}

fn start_app() {
    unsafe {
        let mut app_path = "C:\\Windows\\System32\\notepad.exe\0".encode_utf16().collect::<Vec<_>>();
        app_path.push(0);

        let startup_info = Win32_Threading::STARTUPINFOW { ..Default::default() };
        let mut proc_info = Win32_Threading::PROCESS_INFORMATION { ..Default::default() };
        let mut user_token_handle: Win32_Foundation::HANDLE = Win32_Foundation::HANDLE::default();

        Win32_RemoteDesktop::WTSQueryUserToken(
            Win32_RemoteDesktop::WTSGetActiveConsoleSessionId(),
            &mut user_token_handle
        );

        /* CALL CREATEPROCESSASUSERW */
        let result = Win32_Threading::CreateProcessAsUserW(
            user_token_handle,
            Win32_Core::PCWSTR::from_raw(app_path.as_ptr()), 
            Win32_Core::PWSTR::null(), 
            Option::None, 
            Option::None, 
            Win32_Foundation::TRUE, 
            Win32_Threading::NORMAL_PRIORITY_CLASS, 
            Option::None, 
            Option::None, 
            &startup_info, 
            &mut proc_info
        );

        /* PRINT CREATEPROCESS RESULT */
        let data = "Result: ".to_owned() + result.0.to_string().as_str() + " -> " + Win32_Foundation::GetLastError().0.to_string().as_str();
        fs::write("C:\\spifyresult.txt", data).unwrap_or(println!("Failure"));
    }
}