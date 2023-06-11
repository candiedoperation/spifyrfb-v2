pub mod http {
    /* Define Globals */
    const HTTP_METHODS: [&str; 2] = ["GET", "POST"];

    pub fn get_version(http_request: Vec<&str>) -> String {
        let mut http_version: String = String::from("");
        for header in http_request {
            let version_index = header.find("HTTP/");
            if version_index.is_some() {
                /* Example: GET /home HTTP/1.1 */
                let version_index = version_index.unwrap();
                http_version = header[version_index..].to_string();
                break;
            }
        }
    
        /* Send HTTP Version Reply */
        http_version
    }

    pub fn get_method(http_request: Vec<&str>) -> String {
        let mut http_method: String = String::from("");
        for header in http_request {
            for method in HTTP_METHODS {
                let method_index = header.find(method);
                if method_index.is_some() {
                    http_method = method.to_string();
                    break;
                }
            }

            if http_method != "" {
                break;
            }
        }

        http_method
    }

    pub fn get_websocket_key(http_request: Vec<&str>) -> String {
        let mut websocket_key: String = String::from("");
        for header in http_request {
            let key_identifier = "Sec-WebSocket-Key: ";
            let key_index = header.find(key_identifier);

            if key_index.is_some() {
                websocket_key = header[key_identifier.len()..].to_string();
                break;
            }
        }

        websocket_key
    }

    pub fn response_from_headers(http_headers: Vec<&str>) -> String {
        let mut http_response: String = String::from("");
        for header in http_headers {
            http_response.push_str(format!("{}\r\n", header).as_str());
        }

        /* Add final \r\n to indicate response end */
        http_response.push_str("\r\n");
        http_response
    }
}

pub mod websocket {
    use base64::{engine::general_purpose, Engine};
    use sha1::{Sha1, Digest};

    /* Define Constants */
    const WEBSOCKET_MAGIC_STRING: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

    pub struct OPCODE;
    impl OPCODE {
        pub const CONTINUATION_FRAME: u32 = 0x0;
        pub const TEXT_FRAME: u32 = 0x1;
        pub const BINARY_FRAME: u32 = 0x2;
        pub const CONNECTION_CLOSE: u32 = 0x8;
        pub const PING: u32 = 0x9;
        pub const PONG: u32 = 0xA;
    }

    pub fn get_accept_key(mut websocket_key: String) -> String {
        /* Append Magic String to Client's Websocket Key */
        websocket_key.push_str(WEBSOCKET_MAGIC_STRING);

        /* SHA1 hashes are 20bytes */
        let mut hasher = Sha1::new();
        hasher.update(websocket_key.as_bytes());
        let sha1_hash: &[u8] = &hasher.finalize()[..];

        /* Base64 Encode the SHA-1 Hash */
        let b64 = general_purpose::STANDARD.encode(sha1_hash);
        b64
    }
}