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

use spifyrfb_protocol::info;
use spifyrfb_protocol::server::{RFBAuthentication, VNCAuth};
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("{}", info::license());
    println!("Version: {}, OS: {}", info::srv_version(), env::consts::OS);

    let mut launch_ip: String = String::from("");
    let mut websocket_ip: String = String::from("");
    let mut websocket_secure: bool = false;
    let mut authentication: Option<RFBAuthentication> = Option::None;

    for arg in env::args_os() {
        if arg.to_string_lossy().starts_with("--ip=") {
            launch_ip = String::from(arg.to_string_lossy().replace("--ip=", "").trim());
        } else if arg.to_string_lossy().starts_with("--ws=") {
            websocket_ip = String::from(arg.to_string_lossy().replace("--ws=", "").trim());
        } else if arg.to_string_lossy().starts_with("--ws-secure") {
            websocket_secure = true;
        } else if arg.to_string_lossy().starts_with("--vnc-auth=") {
            let security_key = String::from(arg.to_string_lossy().replace("--vnc-auth=", ""));
            authentication = Option::Some(RFBAuthentication::Vnc(VNCAuth {
                security_key: security_key.as_bytes()[0..8].try_into().unwrap()
            }));
        }
    }

    /* CREATE PROTOCOL SERVER WITH LAUNCH IP */
    spifyrfb_protocol::server::create(
        launch_ip,
        if websocket_ip == "" {
            Option::None
        } else {
            Option::Some((websocket_ip, websocket_secure))
        },
        authentication
    )
    .await
    .unwrap_or({});

    Ok(())
}
