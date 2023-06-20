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

#[cfg(target_os = "windows")]
mod win32;

#[cfg(target_os = "linux")]
mod x11;

pub mod server;
pub mod info {
    pub fn license() -> String {
        String::from("\nSpifyRFB  Copyright (C) 2023  Atheesh Thirumalairajan\nThis program comes with ABSOLUTELY NO WARRANTY; for details type `show w'.\nThis is free software, and you are welcome to redistribute it\nunder certain conditions; type `show c' for details.\n")
    }

    pub fn srv_version() -> String {
        String::from("0.1.0")
    }
}

pub mod debug {
    use std::time::{SystemTime, UNIX_EPOCH, Duration};

    pub fn l1(out: String) {
        println!("{}", out);
    }

    pub fn time_now() -> SystemTime {
        SystemTime::now()
    }

    pub fn time_since_epoch() -> Duration {
        time_now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
    }
}

pub mod authenticate {
    use sha2::Digest;
    use sha2::Sha256;

    use crate::{config, server::parser};

    pub fn server(hash: String) -> bool {
        /* Define Variables */
        let mut server_paired = false;
        let config = config::read();

        for server_hash in config.paired_servers {
            /* Hash Server Key */
            let mut hasher = Sha256::new();
            hasher.update(hash.clone());
            let hash_control = format!("{:X}", hasher.finalize());
            
            if server_hash.eq_ignore_ascii_case(&hash_control) {
                server_paired = true;
                break;
            }
        }

        /* Return Result */
        server_paired
    }

    pub fn server_from_headers(lossy_request: Vec<&str>) -> bool {
        let server_key 
                = parser::http::get_header(lossy_request, String::from("pairkey: "));
        
        if server_key.is_some() { return server(server_key.unwrap()); }
        else { return false; }
    }

    pub fn server_from_json(json: serde_json::Value) -> bool {
        match json {
            serde_json::Value::Object(json) => {
                let md5 = json.get("md5");
                if md5.is_some() {
                    let md5 = md5.unwrap();
                    match md5 {
                        serde_json::Value::String(md5) => {
                            return server(md5.to_owned());
                        },
                        _ => {
                            /* md5 is Invalid */
                            return false;
                        }
                    }
                } else {
                    /* md5 is Empty */
                    return false;
                }
            },
            _ => {
                /* Invalid Structure */
                return false;
            }
        }
    }
}

pub mod config {
    use std::{fs, env, path::PathBuf};
    use serde::{Serialize, Deserialize};
    use serde_json::json;

    #[derive(Serialize, Deserialize, Default)]
    pub struct SpifyConfig {
        pub paired_servers: Vec<String>
    }

    fn create(path: PathBuf) -> SpifyConfig {
        let spify_config: SpifyConfig = Default::default();
        fs::write(path, json!(spify_config).to_string()).unwrap();
        spify_config
    }

    pub fn read() -> SpifyConfig {
        /* Read Config File to String */
        let mut spify_installpath = env::current_exe().unwrap();
        spify_installpath.set_file_name("config.json");
        
        let config = fs::read_to_string(spify_installpath.clone());
        if config.is_ok() {
            let config = config.unwrap();
            let config_json: SpifyConfig = serde_json::from_str(&config).unwrap();
            return config_json;
        } else {
            return create(spify_installpath.clone());
        }
    }

    pub fn new_paired_server(md5: String) {
        let mut config = read();
        config.paired_servers.push(md5);
        
        /* Create JSON */
        let config_json = serde_json::to_string(&config).unwrap();
        fs::write("config.json", config_json).unwrap();
    }
}