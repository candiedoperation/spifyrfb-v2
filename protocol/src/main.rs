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

use std::env;
use std::error::Error;
use spifyrfb_protocol::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("{}", info::license());
    println!("Version: {}, OS: {}", info::srv_version(), env::consts::OS);

    let mut launch_ip: String = String::from("");
    let mut websocket_ip: String = String::from("");

    for arg in env::args_os() {
        if arg.to_string_lossy().starts_with("--ip=") {
            launch_ip = String::from(arg.to_string_lossy().replace("--ip=", "").trim());
        } else if arg.to_string_lossy().starts_with("--ws=") {
            websocket_ip = String::from(arg.to_string_lossy().replace("--ws=", "").trim());
        }
    }

    /* CREATE PROTOCOL SERVER WITH LAUNCH IP */
    spifyrfb_protocol::server::create(
        launch_ip,
        if websocket_ip == "" { Option::None } else { Option::Some(websocket_ip) }
    ).await.unwrap_or({});
    
    Ok(())
}