use super::ToU16Vec;
use windows::core as Win32_Core;
use windows::Win32::System::Pipes as Win32_Pipes;
use windows::Win32::Foundation as Win32_Foundation;
use windows::Win32::System::Threading as Win32_System;
use windows::Win32::Storage::FileSystem as Win32_Filesystem;

fn create_ipc_server() {
    unsafe {
        let pipe_name = String::from(r"\\.\pipe\spifywin32daemonpipe");
        let buf_size: u32 = 1024; /* 1KB */
        
        loop {
            let ipc_pipe = Win32_Pipes::CreateNamedPipeW(
                Win32_Core::PCWSTR::from_raw(String::to_u16_vec(pipe_name.clone()).as_mut_ptr()), 
                Win32_Filesystem::FILE_FLAGS_AND_ATTRIBUTES(0x00000003), /* PIPE_ACCESS_DUPLEX */
                Win32_Pipes::PIPE_TYPE_MESSAGE | Win32_Pipes::PIPE_TYPE_MESSAGE | Win32_Pipes::PIPE_WAIT | Win32_Pipes::PIPE_REJECT_REMOTE_CLIENTS, 
                Win32_Pipes::PIPE_UNLIMITED_INSTANCES, 
                buf_size, 
                buf_size, 
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
                        let thread_handle = Win32_System::CreateThread(
                            Option::None, 
                            0, 
                            lpstartaddress, 
                            Some(&ipc_pipe_handle as *mut _), 
                            dwcreationflags, 
                            lpthreadid
                        );
                    } else {
                        println!("IPC Pipe Client Connection Failed");
                    }
                }
            }
        }
    }
}