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
use spifyrfb_daemon::webapi;

#[tokio::main]
async fn main() {
    /* Spawn WebAPI Service */
    tokio::spawn(async {
        webapi::create().await;
    });

    match env::consts::OS {
        "windows" => {
            #[cfg(target_os = "windows")]
            spifyrfb_daemon::windows::create().await;
        }
        os => {
            println!("OS Not Supported: {}", os);
        }
    }
}