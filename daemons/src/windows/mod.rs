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
use std::fs::OpenOptions;
use std::io::Write;
use std::mem;

use windows::core as Win32_Core;
use windows::Win32::Security as Win32_Security;
use windows::Win32::Foundation as Win32_Foundation;
use windows::Win32::System::Services as Win32_Services;
use windows::Win32::System::Threading as Win32_Threading;
use windows::Win32::System::RemoteDesktop as Win32_RemoteDesktop;
use windows::Win32::System::Diagnostics::ToolHelp as Win32_ToolHelp;

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
        let mut output_file = OpenOptions::new().append(true).write(true).open("C:\\spifyresult2.txt").unwrap();
        let mut app_path = "C:\\Windows\\System32\\cmd.exe\0".encode_utf16().collect::<Vec<_>>();
        app_path.push(0);

        /* GET PROCESS ID OF winlogon.exe */
        let snapshot_handle = 
            Win32_ToolHelp::CreateToolhelp32Snapshot(
                Win32_ToolHelp::TH32CS_SNAPPROCESS, 
                0
        ).unwrap();

        let mut winlogon_process32_ids: Vec<u32> = vec![];
        let mut process32: Win32_ToolHelp::PROCESSENTRY32 = Win32_ToolHelp::PROCESSENTRY32 { 
            dwSize: mem::size_of::<Win32_ToolHelp::PROCESSENTRY32>() as u32, 
            ..Default::default()     
        };

        if Win32_ToolHelp::Process32First(snapshot_handle, &mut process32) == Win32_Foundation::TRUE {
            loop {
                if String::from_utf8_lossy(&process32.szExeFile).to_lowercase().contains("winlogon.exe") == true {
                    winlogon_process32_ids.push((&process32.th32ProcessID).to_owned());
                }

                if Win32_ToolHelp::Process32Next(snapshot_handle, &mut process32) == Win32_Foundation::TRUE {
                    continue;
                } else {
                    break;
                }
            }
        }

        /* Find which winlogon is a part of current Terminal Service Session */
        /* use ProcessIdToSessionId() */
        let winlogon_process_id = winlogon_process32_ids[0];

        let mut winlogin_process_handle: Win32_Foundation::HANDLE = Win32_Foundation::HANDLE::default();
        Win32_Threading::OpenProcessToken(
            Win32_Threading::OpenProcess(
                Win32_Threading::PROCESS_ALL_ACCESS, 
                Win32_Foundation::FALSE, 
                winlogon_process_id
            ).unwrap(), 
            Win32_Security::TOKEN_ALL_ACCESS,
            &mut winlogin_process_handle
        );

        /* 
            let mut user_token_handle: Win32_Foundation::HANDLE = Win32_Foundation::HANDLE::default();
            Win32_RemoteDesktop::WTSQueryUserToken(
                Win32_RemoteDesktop::WTSGetActiveConsoleSessionId(),
                &mut user_token_handle
            ); 
        */

        let mut startup_info = Win32_Threading::STARTUPINFOW { ..Default::default() };
        let mut proc_info = Win32_Threading::PROCESS_INFORMATION { ..Default::default() };
        let mut lp_desktop = String::from(r"winsta0\default").encode_utf16().collect::<Vec<_>>(); lp_desktop.push(0);
        startup_info.lpDesktop = Win32_Core::PWSTR::from_raw(lp_desktop.as_mut_ptr());

        /* CALL CREATEPROCESSASUSERW */
        let result = Win32_Threading::CreateProcessAsUserW(
            winlogin_process_handle,
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
        output_file.write_all(data.as_bytes()).unwrap();
    }
}