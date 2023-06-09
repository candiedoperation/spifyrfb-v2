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

use std::collections::HashMap;
use std::ffi::c_void;
use std::mem;
use std::ptr;
use std::sync::RwLock;

use once_cell::sync::Lazy;
use windows::core as Win32_Core;
use windows::Win32::Security as Win32_Security;
use windows::Win32::Foundation as Win32_Foundation;
use windows::Win32::System::Services as Win32_Services;
use windows::Win32::Networking::WinSock as Win32_WinSock;
use windows::Win32::System::Threading as Win32_Threading;
use windows::Win32::System::RemoteDesktop as Win32_RemoteDesktop;
use windows::Win32::System::Diagnostics::ToolHelp as Win32_ToolHelp;
use windows::Win32::UI::WindowsAndMessaging as Win32_WindowsAndMessaging;

use crate::debug;
use crate::ipc_server;
use crate::ipc_server::IpcEvent;
use crate::ipc_server::event;
use crate::webapi;
use crate::webapi::WebApiSession;

struct SpifyRFBService;
impl SpifyRFBService {
    const SERVICE_NAME: &str = "spifyrfb-daemon";
    const SERVICE_TYPE: Win32_Services::ENUM_SERVICE_TYPE = Win32_Services::SERVICE_WIN32_OWN_PROCESS;
    const SERVICE_CONTROLS: u32 = Win32_Services::SERVICE_ACCEPT_NETBINDCHANGE | Win32_Services::SERVICE_ACCEPT_STOP | Win32_Services::SERVICE_ACCEPT_SESSIONCHANGE;
}

#[derive(Clone, Debug)]
struct SpifyRFBProtocolInstance {
    ip: String,
    ws: String,
    ws_secure: bool,
    vnc_authentication: String,
    wts_info: Win32_RemoteDesktop::WTSINFOW,
    process_info: Win32_Threading::PROCESS_INFORMATION
}

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

/* STATICS FOR PROCESS TRACKING */
static mut SERVICE_HANDLER: Win32_Services::SERVICE_STATUS_HANDLE = Win32_Services::SERVICE_STATUS_HANDLE(0);
static WTS_SESSIONS: Lazy<RwLock<HashMap<u32, SpifyRFBProtocolInstance>>> = Lazy::new(|| { RwLock::new(HashMap::new()) } );
static DAEMON_LISTENIP: Lazy<String> = Lazy::new(|| { String::from("127.0.0.1:39281") });

pub(crate) fn webapi_getsessions() -> Vec<webapi::WebApiSession> {
    let wts_session_lock = WTS_SESSIONS.read().unwrap();
    let mut webapi_sessions = vec![];

    for pid in wts_session_lock.keys() {
        let wts_session = &wts_session_lock[pid];
        let domain = wts_session.wts_info.Domain.clone();
        let username = wts_session.wts_info.UserName.clone();
        
        /* Remove Null Termination in domain and username */
        let valid_domain = domain.iter().position(|&c| c == '\u{0000}' as u16 ).unwrap_or(domain.len());
        let valid_uname = username.iter().position(|&c| c == '\u{0000}' as u16 ).unwrap_or(username.len());

        let formatted_username: String;
        let domain_string = String::from_utf16_lossy(&domain[..valid_domain]);
        if domain_string.eq_ignore_ascii_case(get_hostname().as_str()) == false {
            formatted_username = format!(
                "{} ＠ {}",
                String::from_utf16_lossy(&username[..valid_uname]),
                String::from_utf16_lossy(&domain[..valid_domain])
            );
        } else {
            formatted_username = String::from_utf16_lossy(&username[..valid_uname]);
        }

        webapi_sessions.push(
            WebApiSession {
                pid: pid.to_owned(),
                ip: wts_session.ip.clone(),
                ws: wts_session.ws.clone(),
                ws_secure: wts_session.ws_secure,
                username: formatted_username,
                logontime: (wts_session.wts_info.ConnectTime / 10000000 - 11644473600),
            }
        );
    }

    /* Send Sessions */
    return webapi_sessions;
}

pub(crate) fn get_hostname() -> String {
    unsafe {
        let mut hostname: [u16; 15] = [0; 15];
        Win32_WinSock::GetHostNameW(&mut hostname);
        let valid_hostname = hostname.iter().position(|&c| c as u8 == b'\0' ).unwrap_or(hostname.len());
        String::from_utf16_lossy(&hostname[0..valid_hostname])
    }
}

pub async fn create() {
    unsafe {
        /* Register for IPC Communication Events */
        event::register(IpcEvent::HELLO, process_hello).await;
        event::register(IpcEvent::IP_UPDATE, process_ipupdate).await;
        event::register(IpcEvent::DISCONNECT, process_disconnect).await;

        tokio::spawn(async {
            /* Create IPC Server Instance */
            ipc_server::create(DAEMON_LISTENIP.to_string()).await.unwrap();
        });

        /* Init Windows Service Table Entry */
        let service_start_table = Win32_Services::SERVICE_TABLE_ENTRYW {
            lpServiceName: Win32_Core::PWSTR(SpifyRFBService::SERVICE_NAME.to_owned().as_mut_ptr().cast()),
            lpServiceProc: Some(start)
        };

        /* DISPATCH THE SERVICE */
        Win32_Services::StartServiceCtrlDispatcherW(&service_start_table);
    }
}

fn process_ipupdate(data: String) {
    let data: Vec<&str> = data.split("\r\n").collect();
    let pid: u32 = data[0].parse().unwrap();
    let update = data[1];

    /* Get WTS_SESSION LOCK */
    let mut wts_session_lock = WTS_SESSIONS.write().unwrap();
    let spawnparameters = wts_session_lock.get_mut(&pid);

    if spawnparameters.is_some() {
        let spawnparameters = spawnparameters.unwrap();
        if update == "ws" {
            let ws_ip = data[2];
            spawnparameters.ws = ws_ip.to_string();
            spawnparameters.ws_secure = false;
        } else if update == "wss" {
            let wss_ip = data[2];
            spawnparameters.ws = wss_ip.to_string();
            spawnparameters.ws_secure = true;
        }
    }
}

fn process_hello(data: String) {
    let data: Vec<&str> = data.split("\r\n").collect();
    let pid: u32 = data[0].parse().unwrap();
    let tcp_address = data[1];

    let mut wts_session_lock = WTS_SESSIONS.write().unwrap();
    let spawnparameters = wts_session_lock.get(&pid);
    if spawnparameters.is_some() {
        let mut spawnparameters = spawnparameters.unwrap().clone();
        spawnparameters.ip = tcp_address.to_string();

        /* Update WTS Session */
        wts_session_lock.insert(pid, spawnparameters);    
    }
}

fn process_disconnect(pid: String) {
    let mut wts_session_lock = WTS_SESSIONS.write().unwrap();
    wts_session_lock.remove(&pid.parse().unwrap());
}

unsafe fn get_wts_session_info(session_id: u32) -> Win32_RemoteDesktop::WTSINFOW {
    let mut session_info_ptr: Win32_Core::PWSTR = Win32_Core::PWSTR(ptr::null_mut());
    let mut session_info_bytes: u32 = 0;

    Win32_RemoteDesktop::WTSQuerySessionInformationW(
        Win32_RemoteDesktop::WTS_CURRENT_SERVER_HANDLE, 
        session_id, 
        Win32_RemoteDesktop::WTSSessionInfo, 
        &mut session_info_ptr,
        &mut session_info_bytes
    );

    let session_info = *(session_info_ptr.0 as *const Win32_RemoteDesktop::WTSINFOW);
    Win32_RemoteDesktop::WTSFreeMemory(session_info_ptr.0 as _);
    return session_info;
}

fn create_wts_session(
    process_info: Win32_Threading::PROCESS_INFORMATION, 
    wts_info: Win32_RemoteDesktop::WTSINFOW
) {
    /* Release Lock and Update WTS Sessions */
    let mut wts_session_lock = WTS_SESSIONS.write().unwrap();
    wts_session_lock.insert(process_info.dwProcessId, SpifyRFBProtocolInstance {
        ip: String::from(""),
        ws: String::from(""),
        ws_secure: false,
        vnc_authentication: String::from(""),
        wts_info,
        process_info,
    });
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

            /* Enumerate Sessions and Spawn Protocol for Each Session */
            let mut wts_sessions: *mut Win32_RemoteDesktop::WTS_SESSION_INFOW = ptr::null_mut();
            let mut wts_sessions_length: u32 = 0;

            let sessions_result = Win32_RemoteDesktop::WTSEnumerateSessionsW(
                Win32_RemoteDesktop::WTS_CURRENT_SERVER_HANDLE, 
                0, /* Reserved Parameter must be 0 */
                1, /* Version Parameter must be 1 */
                &mut wts_sessions, 
                &mut wts_sessions_length
            );

            if sessions_result == Win32_Foundation::TRUE {
                let wts_sessions = Vec::from_raw_parts(
                    wts_sessions, 
                    wts_sessions_length as usize, 
                    wts_sessions_length as usize
                );

                for wts_session in wts_sessions {
                    if wts_session.SessionId != 0 {
                        create_wts_session(
                            spawn_spifyrfb_protcol(wts_session.SessionId), 
                            get_wts_session_info(wts_session.SessionId)
                        );
                    }
                }
            }
        },
        Err(_) => {
            /* RETURN SERVICE FAILURE */
            return;
        }
    } 
}

unsafe extern "system" fn event_handler(control: u32, control_event: u32, control_data: *mut c_void, _control_context: *mut c_void) -> u32 {
    match control {
        Win32_Services::SERVICE_CONTROL_STOP => {
            let mut service_status = Win32_Services::SERVICE_STATUS {
                dwServiceType: SpifyRFBService::SERVICE_TYPE,
                dwControlsAccepted: SpifyRFBService::SERVICE_CONTROLS,
                dwCheckPoint: 0,
                dwCurrentState: Win32_Services::SERVICE_STOP_PENDING,
                dwWin32ExitCode: Win32_Foundation::NO_ERROR.0,
                dwServiceSpecificExitCode: 0,
                ..Default::default()
            };

            /* UPDATE FUTURE STATUS HANDLER AND RUNNING STATUS WITH Win32 SERVICES */
            Win32_Services::SetServiceStatus(SERVICE_HANDLER, &service_status);

            let wts_sessions = WTS_SESSIONS.read().unwrap();
            for wts_sessionkey in wts_sessions.keys() {
                /* IPC Closes Processes when spawned with the Daemon Flag */
                let wts_session = wts_sessions.get(wts_sessionkey).unwrap();
                Win32_Foundation::CloseHandle(wts_session.process_info.hProcess);
                Win32_Foundation::CloseHandle(wts_session.process_info.hThread);
            }

            /* SET STOP STATUS AND EXIT GRACEFULLY */
            service_status.dwCurrentState = Win32_Services::SERVICE_STOPPED;
            Win32_Services::SetServiceStatus(SERVICE_HANDLER, &service_status);
            Win32_Foundation::NO_ERROR.0
        },
        Win32_Services::SERVICE_CONTROL_SESSIONCHANGE => {
            /* MATCH SESSION CHANGE CODES */
            let wts_session = 
                *(control_data as *mut Win32_RemoteDesktop::WTSSESSION_NOTIFICATION);
            
            match control_event {
                Win32_WindowsAndMessaging::WTS_SESSION_LOGON => {
                    /* Create Spify Protocol Instance, Get ProcessInfo */
                    create_wts_session(
                        spawn_spifyrfb_protcol(wts_session.dwSessionId), 
                        get_wts_session_info(wts_session.dwSessionId)
                    );

                    /* Return No Errors */
                    Win32_Foundation::NO_ERROR.0
                },
                _ => {
                    Win32_Foundation::NO_ERROR.0
                }
            }
        },
        _ => {
            /* DO NOTHING IF EVENT IS NOT RECOGNIZED */
            Win32_Foundation::NO_ERROR.0
        }
    }
}

fn spawn_spifyrfb_protcol(session_id: u32) -> Win32_Threading::PROCESS_INFORMATION{
    unsafe {
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
                /* Check if this process is WINLOGON.EXE */
                if String::from_utf8_lossy(&process32.szExeFile).to_lowercase().contains("winlogon.exe") == true {
                    winlogon_process32_ids.push((&process32.th32ProcessID).to_owned());
                }

                /* Check if Next Process Exists */
                if Win32_ToolHelp::Process32Next(snapshot_handle, &mut process32) == Win32_Foundation::TRUE {
                    continue;
                } else {
                    break;
                }
            }
        }

        /* Find which winlogon.exe is a part of current Terminal Service Session */
        let mut winlogon_process_id = winlogon_process32_ids[0];
        for process32_id in winlogon_process32_ids {
            let mut session: u32 = 0;
            Win32_RemoteDesktop::ProcessIdToSessionId(
                process32_id, 
                &mut session
            );

            if session == session_id {
                winlogon_process_id = process32_id;
            }
        }

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
        
        let mut startup_info = Win32_Threading::STARTUPINFOW { ..Default::default() };
        let mut proc_info = Win32_Threading::PROCESS_INFORMATION::default();
        let mut lp_desktop = String::from(r"winsta0\default").encode_utf16().collect::<Vec<_>>(); lp_desktop.push(0);
        startup_info.lpDesktop = Win32_Core::PWSTR::from_raw(lp_desktop.as_mut_ptr());

        /* Create App Path String, Set Console Visibility Based on Debug Flag */
        let app_path = format!("spifyrfb-protocol.exe --ip=0.0.0.0:0 --wss=0.0.0.0:0 --spify-daemon={}\0", DAEMON_LISTENIP.to_string());
        let dw_creationflags = if debug::ENABLED == true {
            Win32_Threading::NORMAL_PRIORITY_CLASS
        } else {
            Win32_Threading::NORMAL_PRIORITY_CLASS | Win32_Threading::CREATE_NO_WINDOW
        };

        /* CALL CREATEPROCESSASUSERW */
        Win32_Threading::CreateProcessAsUserW(
            winlogin_process_handle,
            Win32_Core::PCWSTR::null(), 
            Win32_Core::PWSTR::from_raw(String::to_u16_vec(app_path).as_mut_ptr()),
            Option::None, 
            Option::None, 
            Win32_Foundation::TRUE, 
            dw_creationflags, 
            Option::None, 
            Option::None, 
            &startup_info, 
            &mut proc_info
        );

        return proc_info;
    }
}