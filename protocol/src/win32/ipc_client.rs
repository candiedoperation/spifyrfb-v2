use std::ffi::c_void;
use std::mem;
use crate::api;

use super::ToU16Vec;

use windows::core as Win32_Core;
use windows::Win32::System::Pipes as Win32_Pipes;
use windows::Win32::Foundation as Win32_Foundation;
use windows::Win32::System::Threading as Win32_Threading;
use windows::Win32::Storage::FileSystem as Win32_Filesystem;

/* DEFINE GLOBALS */
static PIPE_NAME: &str = r"\\.\pipe\spifywin32daemonpipe";
static BUF_SIZE: u32 = 1024; /* 1KB */

pub type Win32Ipc = Win32_Foundation::HANDLE;
trait API {
    fn send_ip_address_update(self) -> Win32_Foundation::BOOL;
}

impl API for Win32Ipc {
    fn send_ip_address_update(self) -> Win32_Foundation::BOOL {
        unsafe {
            let pipe_write_string = format!("IPU:{}", api::get_listening_ip_address());
            Win32_Filesystem::WriteFile(
                self,
                Some(pipe_write_string.as_bytes()), 
                Option::None, 
                Option::None /* Not Overlapped */
            )
        }
    }
}

pub fn create() {
    unsafe {
        Win32_Threading::CreateThread(
            Option::None, 
            0, 
            Some(listen), 
            Option::None,
            Win32_Threading::THREAD_CREATION_FLAGS(0), /* The thread runs immediately after creation. */
            Option::None
        ).unwrap();
    }
}

unsafe extern "system" fn listen(_thread_param: *mut c_void) -> u32 {
    let pipe_mode: Win32_Pipes::NAMED_PIPE_MODE;
    let mut pipe_handle: Win32_Foundation::HANDLE;    
    let mut pipe_success: Win32_Foundation::BOOL;

    loop {
        pipe_handle = Win32_Filesystem::CreateFileW(
            Win32_Core::PCWSTR::from_raw(String::to_u16_vec(PIPE_NAME.to_string()).as_ptr()), 
            Win32_Foundation::GENERIC_READ.0 | Win32_Foundation::GENERIC_WRITE.0,
            Win32_Filesystem::FILE_SHARE_MODE(0), /* No Sharing */
            Option::None, /* Default Security Attributes. */ 
            Win32_Filesystem::OPEN_EXISTING, 
            Win32_Filesystem::FILE_FLAGS_AND_ATTRIBUTES(0), /* Default Attributes */
            Option::None /* No Template File */
        ).unwrap();

        if pipe_handle != Win32_Foundation::INVALID_HANDLE_VALUE {
            /* Break if Pipe Handle is Valid */
            break;
        }

        if Win32_Foundation::GetLastError() != Win32_Foundation::ERROR_PIPE_BUSY {
            /* Failed to Open Pipe */
            println!("IPC: Failed to Open Pipe");
        } else {
            println!("IPC: Waiting for Pipe Connection");
            Win32_Pipes::WaitNamedPipeW(
                Win32_Core::PCWSTR::from_raw(String::to_u16_vec(PIPE_NAME.to_string()).as_ptr()),
                Win32_Pipes::NMPWAIT_WAIT_FOREVER
            );
        }
    }

    /* Pipe is connected, Change from default byte-read to message-read mode */
    pipe_mode = Win32_Pipes::PIPE_READMODE_MESSAGE;
    pipe_success = Win32_Pipes::SetNamedPipeHandleState(
        pipe_handle, 
        Some(&pipe_mode), 
        Option::None, /* Don't set max bytes */
        Option::None /* Don't set max time */
    );

    if pipe_success == Win32_Foundation::FALSE {
        println!("IPC: Failed to Change Pipe to Message Read Mode");
        return 0;
    }

    /* Update Globals and send Handshake IP Update */
    api::set_spify_ipc_server(api::SpifyIPCServer::Win32(pipe_handle));
    if pipe_handle.send_ip_address_update() == Win32_Foundation::FALSE {
        println!("IPC: Handshake IP Update Failed");
    }

    loop {
        let mut pipe_request: [u8; 1024] = [0; 1024];
        let mut bytes_read: u32 = 0;

        pipe_success = Win32_Filesystem::ReadFile(
            pipe_handle,
            Some(pipe_request.as_mut_ptr().cast()), 
            BUF_SIZE * mem::size_of::<u8>() as u32, 
            Some(&mut bytes_read),
            Option::None
        );

        if pipe_success == Win32_Foundation::FALSE {
            /* ERROR OCCURED */
            break;
        }

        /* PROCESS RESPONSE */
        let mut process_reply: String = String::from("");
        get_request_response(
            String::from_utf8_lossy(&pipe_request[0..bytes_read as usize]).to_string(), 
            &mut process_reply
        );

        let pipe_success = Win32_Filesystem::WriteFile(
            pipe_handle, 
            Some(process_reply.as_bytes()), 
            Option::None,
            Option::None
        );

        if pipe_success == Win32_Foundation::FALSE {
            /* PIPE FAILED TO WRITE */
            break;
        }
    }

    return 1;
}

fn get_request_response(
    request: String,
    process_reply: &mut String
) {
    *process_reply = String::from("OK2");
    println!("REQUEST: {:?}", request);
}