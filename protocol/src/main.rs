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
use spifyrfb_protocol::server::{RFBAuthentication, VNCAuth, ipc_client, CreateOptions};
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("{}", info::license());
    println!("Version: {}, OS: {}", info::srv_version(), env::consts::OS);

    let mut launch_ip: Option<String> = Option::None;
    let mut websocket_proxy: Option<(String, bool)> = Option::None;
    let mut daemon_ip: Option<String> = Option::None;
    let mut authentication: Option<RFBAuthentication> = Option::None;

    for arg in env::args_os() {
        if arg.to_string_lossy().starts_with("--ip=") {
            launch_ip = Option::Some(String::from(arg.to_string_lossy().replace("--ip=", "").trim()));
        } else if arg.to_string_lossy().starts_with("--ws=") {
            websocket_proxy = Option::Some((String::from(arg.to_string_lossy().replace("--ws=", "").trim()), false));
        } else if arg.to_string_lossy().starts_with("--wss=") {
            websocket_proxy = Option::Some((String::from(arg.to_string_lossy().replace("--wss=", "").trim()), true));
        } else if arg.to_string_lossy().starts_with("--vnc-auth=") {
            let security_key = String::from(arg.to_string_lossy().replace("--vnc-auth=", ""));
            authentication = Option::Some(RFBAuthentication::Vnc(VNCAuth {
                security_key: security_key.as_bytes()[0..8].try_into().unwrap()
            }));
        } else if arg.to_string_lossy().starts_with("--spify-daemon=") {
            let ip = String::from(arg.to_string_lossy().replace("--spify-daemon=", ""));
            daemon_ip = Option::Some(ip.clone());
            tokio::spawn(async {
                /* Connect to Spify Daemon Server */
                ipc_client::connect(ip).await.unwrap();
            });
        }
    }

    let create_options = CreateOptions {
        ip_address: launch_ip.unwrap(),
        ws_proxy: websocket_proxy,
        auth: authentication,
        spify_daemon: daemon_ip.is_some()
    };

    /* CREATE PROTOCOL SERVER WITH LAUNCH IP */
    spifyrfb_protocol::server::create(create_options).await.unwrap_or({});

    Ok(())
}
