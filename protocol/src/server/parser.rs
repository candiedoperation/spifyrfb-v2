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

use std::mem;

pub trait GetBits {
    fn get_bits_be(&self) -> Vec<bool>;
    fn from_bits(bits: Vec<bool>, is_le: bool) -> Self;
    fn get_bits_le(&self) -> Vec<bool> {
        let be_bits = self.get_bits_be();
        be_bits.into_iter().rev().collect()
    }
}

impl GetBits for u8 {
    fn get_bits_be(&self) -> Vec<bool> {
        const BIT_COUNT: usize = mem::size_of::<u8>() * 8;
        let mut bits: Vec<bool> = Vec::with_capacity(BIT_COUNT);
        for index in 0..BIT_COUNT {
            bits.push((self >> index & 1) == 1);
        }

        return bits;
    }

    fn from_bits(bits: Vec<bool>, is_le: bool) -> Self {
        let mut bits_added: Vec<bool> = vec![];
        let sizeof = mem::size_of::<Self>() * 8;
        let mut integer: Self = 0;

        /* Add bit headers for bits less than size */
        if bits.len() < sizeof {
            bits_added = vec![false; sizeof - bits.len()]
        }

        bits_added.extend_from_slice(&bits[..]);
        if is_le == true {
            /* If input is Little Endian, Reverse the bits */
            bits_added = bits_added.into_iter().rev().collect();
        }

        for index in 0..bits_added.len() {
            if bits_added[index] == true {
                integer = integer | (1 << index);
            }
        }

        integer
    }
}

impl GetBits for u16 {
    fn get_bits_be(&self) -> Vec<bool> {
        const BIT_COUNT: usize = mem::size_of::<u16>() * 8;
        let mut bits: Vec<bool> = Vec::with_capacity(BIT_COUNT);
        for index in 0..BIT_COUNT {
            bits.push((self >> index & 1) == 1);
        }

        return bits;
    }

    fn from_bits(bits: Vec<bool>, is_le: bool) -> Self {
        let mut bits_added: Vec<bool> = vec![];
        let sizeof = mem::size_of::<Self>() * 8;
        let mut integer: Self = 0;

        /* Add bit headers for bits less than size */
        if bits.len() < sizeof {
            bits_added = vec![false; sizeof - bits.len()]
        }

        bits_added.extend_from_slice(&bits[..]);
        if is_le == true {
            /* If input is Little Endian, Reverse the bits */
            bits_added = bits_added.into_iter().rev().collect();
        }

        for index in 0..bits_added.len() {
            if bits_added[index] == true {
                integer = integer | (1 << index);
            }
        }

        integer
    }
}

impl GetBits for u64 {
    fn get_bits_be(&self) -> Vec<bool> {
        const BIT_COUNT: usize = mem::size_of::<u64>() * 8;
        let mut bits: Vec<bool> = Vec::with_capacity(BIT_COUNT);
        for index in 0..BIT_COUNT {
            bits.push((self >> index & 1) == 1);
        }

        return bits;
    }

    fn from_bits(bits: Vec<bool>, is_le: bool) -> Self {
        let mut bits_added: Vec<bool> = vec![];
        let sizeof = mem::size_of::<Self>() * 8;
        let mut integer: Self = 0;

        /* Add bit headers for bits less than size */
        if bits.len() < sizeof {
            bits_added = vec![false; sizeof - bits.len()]
        }

        bits_added.extend_from_slice(&bits[..]);
        if is_le == true {
            /* If input is Little Endian, Reverse the bits */
            bits_added = bits_added.into_iter().rev().collect();
        }

        for index in 0..bits_added.len() {
            if bits_added[index] == true {
                integer = integer | (1 << index);
            }
        }

        integer
    }
}

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

pub mod tls {
    use std::{fs, io};
    use rustls::{Certificate, PrivateKey};
    use rustls_pemfile::{certs, rsa_private_keys, pkcs8_private_keys};

    pub fn load_certificates(pem_path: &str) -> Vec<Certificate> {
        let certpem = fs::File::open(pem_path).unwrap();
        let mut certreader = io::BufReader::new(certpem);
        certs(&mut certreader).unwrap().into_iter().map(Certificate).collect()
    }

    pub fn load_privatekey(pem_path: &str) -> PrivateKey {
        let pkeypem = fs::File::open(pem_path).unwrap();
        let mut pkeyreader = io::BufReader::new(pkeypem);
        let mut privatekey = rsa_private_keys(&mut pkeyreader).unwrap();

        /* rsa_private_key() returns [] if key is pkcs8 */
        if privatekey.len() == 0 {
            let pkeypem = fs::File::open(pem_path).unwrap();
            pkeyreader = io::BufReader::new(pkeypem);
            privatekey = pkcs8_private_keys(&mut pkeyreader).unwrap();
        }
        
        PrivateKey(privatekey[0].clone())   
    }
}

pub mod websocket {
    use base64::{engine::general_purpose, Engine};
    use rand::Rng;
    use sha1::{Digest, Sha1};
    use super::GetBits;

    /* Define Constants */
    const WEBSOCKET_MAGIC_STRING: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

    pub struct OPCODE;
    impl OPCODE {
        pub const CONTINUATION_FRAME: u8 = 0x0;
        pub const TEXT_FRAME: u8 = 0x1;
        pub const BINARY_FRAME: u8 = 0x2;
        pub const CONNECTION_CLOSE: u8 = 0x8;
        pub const PING: u8 = 0x9;
        pub const PONG: u8 = 0xA;
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

    fn generate_masking_key() -> [u8; 4] {
        let mut rand_rng = rand::thread_rng();
        (rand_rng.gen::<u32>()).to_be_bytes()
    }

    pub fn unmask_payload(mask_key: [u8; 4], payload: Vec<u8>) -> Vec<u8> {
        let mut decoded_payload: Vec<u8> = Vec::with_capacity(payload.len());
        for index in 0..payload.len() {
            decoded_payload.push(payload[index] ^ mask_key[index % 4]);
        }

        /* Return Decoded Payload */
        return decoded_payload;
    }

    pub fn mask_payload(payload: Vec<u8>) -> Vec<u8> {
        /* TODO */
        vec![]
    }

    pub fn create_frame(payload: Vec<u8>, opcode: u8, secure: bool) -> Vec<u8> {
        /* A new websocket Frame */
        let mut websocket_frame: Vec<u8> = Vec::with_capacity(payload.len() + 15);

        /* First Byte has FIN/RSV(1-3)/OPCODE */
        let mut fin_byte = vec![true; 1];
        fin_byte.extend_from_slice(&[false; 3]);
        fin_byte.extend_from_slice(&opcode.get_bits_le()[4..8]);

        let mut mask_byte = vec![false; 1];
        let masking_key: Option<[u8; 4]>;
        if secure == true {
            mask_byte[0] = true;
            masking_key = Option::Some(generate_masking_key());
        } else {
            masking_key = Option::None;
        }

        /* Add Payload hint to Mask Byte */
        let mut extended_payload: Vec<u8> = vec![];

        if payload.len() < 126 {
            let payload_hint = payload.len() as u8;
            mask_byte.extend_from_slice(&payload_hint.get_bits_le()[1..8]);
        } else if payload.len() <= u16::MAX as usize {
            /* Define Payload Hint and Length */
            let payload_hint = 126_u8;
            let payload_len = payload.len() as u16;

            /* Write Hint and Extended Payload */
            mask_byte.extend_from_slice(&payload_hint.get_bits_le()[1..8]);
            extended_payload.extend_from_slice(&payload_len.to_be_bytes());
        } else if payload.len() <= u64::MAX as usize {
            let payload_hint = 127_u8;
            let payload_len = payload.len() as u64;

            mask_byte.extend_from_slice(&payload_hint.get_bits_le()[1..8]);
            extended_payload.extend_from_slice(&payload_len.to_be_bytes());
        } else { /* USE FIN, TODO in Future */ }

        websocket_frame.push(u8::from_bits(fin_byte, true));
        websocket_frame.push(u8::from_bits(mask_byte, true));
        websocket_frame.extend_from_slice(&extended_payload);

        /* Add Masking Key if Secure */
        if masking_key.is_some() {
            websocket_frame.extend_from_slice(&masking_key.unwrap())
        }
        
        /* Add Payload to Frame */
        if secure == true {
            websocket_frame.extend_from_slice(&mask_payload(payload));
        } else {
            websocket_frame.extend_from_slice(&payload);
        }

        /* Return Created Frame */
        return websocket_frame;
    }
}
