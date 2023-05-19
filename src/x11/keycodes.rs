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
use x11rb::{protocol::xproto, rust_connection::RustConnection};

struct KeyCodeInfo;
impl KeyCodeInfo {
    const MIN_KEYCODE: u8 = 8;
    const MAX_KEYCODE: u8 = 255;
}

pub fn create_keysym_map(x11_connection: &RustConnection) -> HashMap<u32, u8> {
    let mut keysym_keycode_map: HashMap<u32, u8> = HashMap::new();
    let keyboard_mapping_cookie = xproto::get_keyboard_mapping(
        x11_connection,
        KeyCodeInfo::MIN_KEYCODE,
        ((KeyCodeInfo::MAX_KEYCODE as i8 + 1) - KeyCodeInfo::MIN_KEYCODE as i8) as u8,
    );

    let keyboard_mapping_cookie = keyboard_mapping_cookie.unwrap().reply().unwrap();
    let keysyms_per_keycode = keyboard_mapping_cookie.keysyms_per_keycode as usize;
    let valid_keysyms = keyboard_mapping_cookie.keysyms;

    for valid_keysym in valid_keysyms.chunks(keysyms_per_keycode).enumerate() {
        for keysym in valid_keysym.1 {
            if keysym == &x11rb::NO_SYMBOL {
                continue;
            } else {
                keysym_keycode_map.insert(keysym.clone(), valid_keysym.0 as u8 + KeyCodeInfo::MIN_KEYCODE);
            }
        }
    }

    /* RETURN KEYSYM <-> KEYCODE MAP */
    keysym_keycode_map
}
