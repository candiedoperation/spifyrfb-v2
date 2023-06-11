use std::{sync::RwLock, collections::HashMap};

use once_cell::sync::Lazy;

#[derive(Clone, Copy)]
pub struct SpifySession {
    pub zlib_stream: libz_sys::z_stream
}

static mut ACTIVE_SESSIONS: Lazy<RwLock<HashMap<String, SpifySession>>> = Lazy::new(|| {
    RwLock::new(HashMap::new())
});

pub fn new(peer_address: String, session: SpifySession) {
    unsafe {
        let mut session_lock = ACTIVE_SESSIONS.write().unwrap();
        session_lock.insert(peer_address, session);
    }
}

pub fn get(peer_address: String) -> SpifySession {
    unsafe {
        let session_lock = ACTIVE_SESSIONS.read().unwrap();
        session_lock.get(&peer_address).unwrap().clone()
    }
}

pub fn destroy(peer_address: String) {
    unsafe {
        let mut session_lock = ACTIVE_SESSIONS.write().unwrap();
        session_lock.remove(&peer_address);
    }
}

pub fn get_zlib_stream(peer_address: String) -> libz_sys::z_stream {
    unsafe {
        let session_lock = ACTIVE_SESSIONS.read().unwrap();
        session_lock.get(&peer_address).unwrap().zlib_stream        
    }
}

pub fn update_zlib_stream(peer_address: String, zlib_stream: libz_sys::z_stream) {
    unsafe {
        let mut session_lock = ACTIVE_SESSIONS.write().unwrap();
        let session_data = session_lock.get(&peer_address).unwrap().clone();

        session_lock.insert(peer_address.clone(), SpifySession {
            zlib_stream,
            ..session_data
        });
    }
}