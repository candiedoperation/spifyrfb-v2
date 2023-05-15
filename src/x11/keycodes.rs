use x11rb::{rust_connection::RustConnection, protocol::xproto};

pub fn get_x11_keycode(key: u32) -> u8 {
    40
}

pub fn create_keysym_map(x11_connection: &RustConnection) {
    let keyboard_mapping_cookie = xproto::get_keyboard_mapping(
        x11_connection, 
        40, 
        1
    );

    let keyboard_mapping_cookie = keyboard_mapping_cookie.unwrap().reply().unwrap();
    println!("Keysyms: {:?}", keyboard_mapping_cookie.keysyms);
}