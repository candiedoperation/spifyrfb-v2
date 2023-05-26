use std::ffi::c_void;

use windows::core as Win32_Core;
use windows::Win32::System::Pipes as Win32_Pipes;
use windows::Win32::Foundation as Win32_Foundation;
use windows::Win32::System::Threading as Win32_Threading;
use windows::Win32::Storage::FileSystem as Win32_Filesystem;

use super::ToU16Vec;

/* DEFINE GLOBALS */
static PIPE_NAME: &str = r"\\.\pipe\spifywin32daemonpipe";
static BUF_SIZE: u32 = 1024; /* 1KB */

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
    let mut pipe_bytes_read: u32 = 0;
    let mut pipe_bytes_write: u32 = 0;
    let mut pipe_bytes_written: u32 = 0;
    let pipe_mode: Win32_Pipes::NAMED_PIPE_MODE;
    let mut pipe_handle: Win32_Foundation::HANDLE;    
    let mut pipe_success: Win32_Foundation::BOOL = Win32_Foundation::FALSE;

    loop {
        println!("Looping!");
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

    let pipe_write_string = String::from("hehe");
    pipe_success = Win32_Filesystem::WriteFile(
        pipe_handle, 
        Some(pipe_write_string.as_bytes()), 
        Some(&mut pipe_bytes_written), 
        Option::None /* Not Overlapped */
    );

    if pipe_success == Win32_Foundation::FALSE {
        println!("Pipe Write Error");
        return 0;
    }

    return 1;
}