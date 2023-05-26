use std::sync::RwLock;

use once_cell::sync::Lazy;
use crate::win32::ipc_client::Win32Ipc;

#[derive(Clone, Copy)]
pub enum SpifyIPCServer {
    Win32(Win32Ipc),
    None
}

static LISTENING_IP_ADDRESS: Lazy<RwLock<String>> = Lazy::new(|| { RwLock::new(String::from("")) });
static SPIFY_IPC_SERVER: RwLock<SpifyIPCServer> = RwLock::new(SpifyIPCServer::None);

pub unsafe fn get_listening_ip_address() -> String {
    let lock = LISTENING_IP_ADDRESS.read().unwrap();
    lock.to_string()
}

pub fn set_listening_ip_address(ip: String) {
    let mut lock = LISTENING_IP_ADDRESS.write().unwrap();
    *lock = ip;
}

pub fn set_spify_ipc_server(ipc_server: SpifyIPCServer) {
    let mut lock = SPIFY_IPC_SERVER.write().unwrap();
    *lock = ipc_server;
}

pub fn get_spify_ipc_server() -> SpifyIPCServer {
    let lock = SPIFY_IPC_SERVER.read().unwrap();
    *lock
}