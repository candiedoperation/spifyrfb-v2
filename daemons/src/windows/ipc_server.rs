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
use std::slice::from_raw_parts;

use super::ToU16Vec;
use windows::core as Win32_Core;
use windows::Win32::System::Pipes as Win32_Pipes;
use windows::Win32::Foundation as Win32_Foundation;
use windows::Win32::System::Memory as Win32_Memory;
use windows::Win32::System::Threading as Win32_Threading;
use windows::Win32::Storage::FileSystem as Win32_Filesystem;

/* DEFINE GLOBALS */
static PIPE_NAME: &str = r"\\.\pipe\spifywin32daemonpipe";
static BUF_SIZE: u32 = 1024; /* 1KB */

pub struct SpifyIPCClient;
impl SpifyIPCClient {
    pub fn a() {

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
    unsafe {
        loop {
            let ipc_pipe = Win32_Pipes::CreateNamedPipeW(
                Win32_Core::PCWSTR::from_raw(String::to_u16_vec(PIPE_NAME.to_string()).as_mut_ptr()), 
                Win32_Filesystem::FILE_FLAGS_AND_ATTRIBUTES(0x00000003), /* PIPE_ACCESS_DUPLEX */
                Win32_Pipes::PIPE_TYPE_MESSAGE | Win32_Pipes::PIPE_TYPE_MESSAGE | Win32_Pipes::PIPE_WAIT | Win32_Pipes::PIPE_REJECT_REMOTE_CLIENTS, 
                Win32_Pipes::PIPE_UNLIMITED_INSTANCES, 
                BUF_SIZE, 
                BUF_SIZE, 
                0,
                Option::None
            );

            match ipc_pipe {
                Win32_Foundation::INVALID_HANDLE_VALUE => {
                    println!("IPC Pipe Creation Failed");
                    break;
                }
                ipc_pipe_handle => {
                    /* WAIT FOR CLIENT TO CONNECT */
                    let client_connection = Win32_Pipes::ConnectNamedPipe(
                        ipc_pipe_handle, 
                        Option::None
                    );

                    if client_connection == Win32_Foundation::TRUE {
                        /* CLIENT CONNECTED, MOVE TO NEW THREAD */
                        let mut spawn_thread_id = 0;
                        Win32_Threading::CreateThread(
                            Option::None, 
                            0, 
                            Some(handle_ipc_client), 
                            Some(mem::transmute(ipc_pipe_handle)), 
                            Win32_Threading::THREAD_CREATION_FLAGS(0), /*  The thread runs immediately after creation.  */ 
                            Some(&mut spawn_thread_id)
                        ).unwrap();
                    } else {
                        println!("IPC Pipe Client Connection Failed");
                    }
                }
            }
        }
    }

    return 1;
}

unsafe extern "system" fn handle_ipc_client(ipc_param: *mut c_void) -> u32 {
    let heap_handle = Win32_Memory::GetProcessHeap().unwrap();
    let process_request = Win32_Memory::HeapAlloc(heap_handle, Win32_Memory::HEAP_FLAGS(0), BUF_SIZE as usize * mem::size_of::<char>());
    let process_reply = Win32_Memory::HeapAlloc(heap_handle, Win32_Memory::HEAP_FLAGS(0), BUF_SIZE as usize * mem::size_of::<char>());

    let mut pipe_bytes_read: u32 = 0;
    let mut pipe_bytes_reply: u32 = 0;
    let mut pipe_bytes_written: u32 = 0;
    let mut pipe_read_success: Win32_Foundation::BOOL;
    let pipe_handle: Option<Win32_Foundation::HANDLE>;

    /* ERROR CHECKING */
    if ipc_param.is_null() {
        /* THREAD PARAMETER IS NULL */
        if !process_request.is_null() { Win32_Memory::HeapFree(heap_handle, Win32_Memory::HEAP_FLAGS(0), Some(process_request)); }
        if !process_reply.is_null() { Win32_Memory::HeapFree(heap_handle, Win32_Memory::HEAP_FLAGS(0), Some(process_reply)); }
        return 0;
    }

    if process_request.is_null() {
        /* BUFFER ASSIGNED FOR REQUEST IS NULL */
        if process_reply.is_null() { Win32_Memory::HeapFree(heap_handle, Win32_Memory::HEAP_FLAGS(0), Some(process_reply)); }
        return 0;
    }

    if process_reply.is_null() {
        /* BUFFER ASSIGNED FOR REPLY IS NULL */
        if process_request.is_null() { Win32_Memory::HeapFree(heap_handle, Win32_Memory::HEAP_FLAGS(0), Some(process_request)); }
        return 0;
    }

    /* THEAD CAN SUCCESSFULLY RECEIVE MESSAGES, IT'S INITIALIZED */
    pipe_handle = Some(Win32_Foundation::HANDLE(ipc_param as _));
    loop {
        pipe_read_success = Win32_Filesystem::ReadFile(
            pipe_handle.unwrap(), 
            Some(process_request), 
            BUF_SIZE * mem::size_of::<char>() as u32, 
            Some(&mut pipe_bytes_read), 
            Option::None
        );

        if pipe_read_success == Win32_Foundation::FALSE || pipe_bytes_read == 0 {
            if Win32_Foundation::GetLastError() == Win32_Foundation::ERROR_BROKEN_PIPE {
                /* CLIENT HAS DISCONNECTED */
            } else {
                /* SOME OTHER ERROR */
            }

            break;
        }

        /* PROCESS RESPONSE */
        let mut process_reply_string = String::from("");
        get_request_response(
            String::from_raw_parts(process_request as _, pipe_bytes_read as usize, pipe_bytes_read as usize), 
            &mut process_reply_string,
            &mut pipe_bytes_reply
        );

        pipe_read_success = Win32_Filesystem::WriteFile(
            pipe_handle.unwrap(), 
            Some(from_raw_parts(process_reply as _, pipe_bytes_reply as usize)), 
            Some(&mut pipe_bytes_written),
            Option::None
        );

        if pipe_read_success == Win32_Foundation::FALSE || pipe_bytes_reply != pipe_bytes_written {
            /* PIPE FAILED TO WRITE */
            break;
        }
    }

    /* CLIENT HAS DISCONNECTED (or PIPE FAILED), THREAD NEEDS TO BE CLOSED */
    Win32_Filesystem::FlushFileBuffers(pipe_handle.unwrap());
    Win32_Pipes::DisconnectNamedPipe(pipe_handle.unwrap());
    Win32_Foundation::CloseHandle(pipe_handle.unwrap());

    /* FREE REQUEST, REPLY HEAP AND EXIT */
    Win32_Memory::HeapFree(heap_handle, Win32_Memory::HEAP_FLAGS(0), Some(process_request));
    Win32_Memory::HeapFree(heap_handle, Win32_Memory::HEAP_FLAGS(0), Some(process_reply));
    1
}

fn get_request_response(
    request: String,
    process_reply: &mut String,
    pipe_reply_bytes: &mut u32
) {
    let mut output_file = OpenOptions::new().append(true).write(true).open("C:\\spifyresult3.txt").unwrap();
    
    let mut send_reply = |process_reply_string: String| {
        *process_reply = process_reply_string.clone();
        *pipe_reply_bytes = (process_reply_string.as_bytes().len() * mem::size_of::<char>()) as u32;
    };

    if request.starts_with("IPU:") {
        let spifyrfb_protocol_ip = &request.as_bytes()[4..];
        output_file.write_all(spifyrfb_protocol_ip).unwrap();
        output_file.write_all("\n".as_bytes()).unwrap();       
        send_reply(String::from("OK?:1"));
    }
}