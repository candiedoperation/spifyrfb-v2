use std::error::Error;
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncReadExt, AsyncWriteExt}};
use crate::server::parser;

trait GetBits {
    fn get_bits_be(&self) -> Vec<bool>;
    fn from_bits(bits: Vec<bool>) -> Self;
    fn get_bits_le(&self) -> Vec<bool> {
        let be_bits = self.get_bits_be();
        be_bits.into_iter().rev().collect()
    }
}

impl GetBits for u8 {
    fn get_bits_be(&self) -> Vec<bool> {
        let bit_count = self.count_ones() + self.count_zeros();
        let mut bits: Vec<bool> = Vec::with_capacity(bit_count as usize);
        for index in 0..bit_count {
            bits.push((self >> index & 1) == 1);
        }

        return bits;
    }

    fn from_bits(bits: Vec<bool>) -> u8 {
        let mut integer: u8 = 0;
        for index in 0..bits.len() {
            if bits[index] == true {
                integer = integer | (1 << index);
            }
        }

        integer
    }
}

impl GetBits for u16 {
    fn get_bits_be(&self) -> Vec<bool> {
        let bit_count = self.count_ones() + self.count_zeros();
        let mut bits: Vec<bool> = Vec::with_capacity(bit_count as usize);
        for index in 0..bit_count {
            bits.push((self >> index & 1) == 1);
        }

        return bits;
    }

    fn from_bits(bits: Vec<bool>) -> u16 {
        let mut integer: u16 = 0;
        for index in 0..bits.len() {
            if bits[index] == true {
                integer = integer | (1 << index);
            }
        }

        integer
    }
}

impl GetBits for u64 {
    fn get_bits_be(&self) -> Vec<bool> {
        let bit_count = self.count_ones() + self.count_zeros();
        let mut bits: Vec<bool> = Vec::with_capacity(bit_count as usize);
        for index in 0..bit_count {
            bits.push((self >> index & 1) == 1);
        }

        return bits;
    }

    fn from_bits(bits: Vec<bool>) -> u64 {
        let mut integer: u64 = 0;
        for index in 0..bits.len() {
            if bits[index] == true {
                integer = integer | (1 << index);
            }
        }

        integer
    }
}

impl GetBits for u128 {
    fn get_bits_be(&self) -> Vec<bool> {
        let bit_count = self.count_ones() + self.count_zeros();
        let mut bits: Vec<bool> = Vec::with_capacity(bit_count as usize);
        for index in 0..bit_count {
            bits.push((self >> index & 1) == 1);
        }

        return bits;
    }

    fn from_bits(bits: Vec<bool>) -> u128 {
        let mut integer: u128 = 0;
        for index in 0..bits.len() {
            if bits[index] == true {
                integer = integer | (1 << index);
            }
        }

        integer
    }
}

async fn listen_websocket(mut client: TcpStream) {
    /* Split Stream for simulatneous TX/RX */
    let (mut client_rx, mut client_tx) = client.split();

    loop {
        /* Read Websocket Opcode */
        let mut buf: [u8; 2] = [0; 2];
        match client_rx.read_exact(&mut buf).await {
            Ok(0) => {
                /* Client has closed the Connection */
                println!("Websocket Client Closed Connection");
                return;
            },
            Ok(_) => {
                /* Get Opcode */
                let fin_flag = buf[0].get_bits_be()[0];
                let mut opcode = vec![false; 4];
                opcode.extend_from_slice(&buf[0].get_bits_be()[4..7]);
                let opcode: u8 = u8::from_bits(opcode);
                
                /* Find Payload Hint */
                let payload_length: u64;
                let mask_key: Option<[u8; 4]>;
                let mask_hint: bool = buf[1].get_bits_be()[0];

                let mut payload_hint: Vec<bool> = vec![];
                payload_hint.extend_from_slice(&buf[1].get_bits_be()[1..7].to_vec());
                println!("L: {:?}", payload_hint.clone());
                let payload_hint = u8::from_bits(payload_hint);

                if payload_hint < 126 {
                    /* Payload length is same as hint (if = or < 125) */
                    payload_length = payload_hint as u64;
                } else if payload_hint == 126 {
                    /* Read next 16 bits */
                    let mut payload_buf: [u8; 2] = [0; 2];
                    client_rx.read_exact(&mut payload_buf).await.unwrap();
                    payload_length = u16::from_be_bytes(payload_buf) as u64;
                } else if payload_hint == 127 {
                    /* Read next 64 bits */
                    let mut payload_buf: [u8; 8] = [0; 8];
                    client_rx.read_exact(&mut payload_buf).await.unwrap();
                    payload_length = u64::from_be_bytes(payload_buf);
                } else {
                    payload_length = 0;
                }

                if mask_hint == true {
                    /* Next 32 bits is Mask Key */
                    let mut mask_key_buf: [u8; 4] = [0; 4];
                    client_rx.read_exact(&mut mask_key_buf).await.unwrap();
                    mask_key = Option::Some(mask_key_buf);
                } else {
                    /* Set Mask Key to None */
                    mask_key = Option::None;
                }

                println!("STATS: {}, {}", payload_length, fin_flag);

                let mut payload: Vec<u8> = vec![0; payload_length as usize];
                let payload_buffer = &mut payload[..];
                client_rx.read_exact(payload_buffer).await.unwrap();

                println!("PAYLOAD: {:?}", payload);

                match opcode {
                    _ => {
                        println!("Invalid OPCODE: {}", opcode);
                    }
                }
            },
            Err(_) => {
                /* Client has Disconnected, Unexpected Error */
                println!("Websocket Client Disconnected (ERR)");
                return;
            },
        }
    }
}

async fn handle_wsclient(mut client: TcpStream) {
    let mut buf: [u8; 32768] = [0; 32768];
    let bits_read = client
    .read(&mut buf)
    .await
    .unwrap();

    let handshake_request = String::from_utf8_lossy(&buf[..bits_read]);
    let handshake_request: Vec<&str> = handshake_request.split("\r\n").collect();
    
    /* Debugging */
    println!("Request: {:?}", handshake_request);

    let handshake_request_version = parser::http::get_version(handshake_request.clone());
    let handshake_request_method = parser::http::get_method(handshake_request.clone());
    if handshake_request_version == "HTTP/1.1" && handshake_request_method == "GET" {
        /* This is a valid Websocket Handshake Request, Check WS Version Support */
        let websocket_key = parser::http::get_websocket_key(handshake_request.clone());
        let websocket_accept_key = parser::websocket::get_accept_key(websocket_key);
        
        /* Generate Handshake Response */
        let handshake_response = parser::http::response_from_headers([
            "HTTP/1.1 101 Switching Protocols",
            "Upgrade: websocket",
            "Connection: Upgrade",
            format!("Sec-WebSocket-Accept: {}", websocket_accept_key).as_str()
        ].to_vec());

        /* Send Response and Complete Handshake */
        client
        .write(handshake_response.as_bytes())
        .await
        .unwrap();

        /* Handshake Response Sent, Proceed Further */
        listen_websocket(client).await;
    } else {
        /* Send 400 (Bad Request) */
    }
}

pub async fn create(tcp_address: String, proxy_address: String) -> Result<(), Box<dyn Error>> {
    match TcpListener::bind(tcp_address).await {
        Ok(listener) => {
            println!("SpifyRFB Websocket Communications at {:?}\n", listener.local_addr().unwrap());

            loop {
                let (client, _) = listener.accept().await?;
                tokio::spawn(async move {
                    /* Init Handshake */
                    println!("Connection Established: {:?}", client);
                    handle_wsclient(client).await;
                });
            }
        },
        Err(err) => {
            println!("Websocket IP Address Binding Failed -> {}", err.to_string());
            Err(err.into())
        }
    }
}