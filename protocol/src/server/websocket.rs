use std::{error::Error, mem};
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncReadExt, AsyncWriteExt}};
use crate::{server::parser, debug};

use super::parser::websocket::OPCODE;

trait GetBits {
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

async fn listen_websocket(mut client: TcpStream) {
    /* Split Stream for simulatneous TX/RX */
    let (mut client_rx, mut client_tx) = client.split();
    let mut fin_payload: Vec<u8> = vec![];
    let mut fin_payload_opcode: u8 = 0;

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
                /*
                    We create slices using a range within brackets by specifying 
                    [starting_index..ending_index], where starting_index is the 
                    first position in the slice and ending_index is one more than 
                    the last position in the slice.
                */

                /* Get Opcode */
                let fin_flag = buf[0].get_bits_le()[0];
                let mut opcode: u8 = u8::from_bits(buf[0].get_bits_le()[4..8].to_vec(), true);
                
                /* Find Payload Hint */
                let payload_length: u64;
                let mask_key: Option<[u8; 4]>;
                let mask_hint: bool = buf[1].get_bits_le()[0];
                let payload_hint = u8::from_bits(buf[1].get_bits_le()[1..8].to_vec(), true);

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

                /* Read the Payload */
                let mut payload: Vec<u8> = vec![0; payload_length as usize];
                let payload_buffer = &mut payload[..];
                client_rx.read_exact(payload_buffer).await.unwrap();

                /* Decode the Payload, if mask was set */
                if mask_key.is_some() {
                    let payload_mask = mask_key.unwrap();
                    let mut decoded_payload: Vec<u8> = Vec::with_capacity(payload.len());

                    for index in 0..payload.len() {
                        decoded_payload.push(
                            payload[index] ^ payload_mask[index % 4]
                        );
                    }

                    /* Update the Payload */
                    payload = decoded_payload;
                }

                if fin_flag == false {
                    /* Extend FIN Payload */
                    if opcode != OPCODE::CONTINUATION_FRAME { fin_payload_opcode = opcode; }
                    fin_payload.extend_from_slice(&payload[..]);

                    /* Do not process the Payload */
                    continue;
                } else if fin_flag == true && opcode == OPCODE::CONTINUATION_FRAME  {
                    fin_payload.extend_from_slice(&payload[..]);
                    payload = fin_payload.clone();
                    opcode = fin_payload_opcode;

                    /* Clear FIN Payload for next FIN Message */
                    fin_payload = vec![];
                }

                /* Process the Payload */
                match opcode {
                    OPCODE::TEXT_FRAME => {
                        debug::l1(format!("PAYLOAD:\n{}", String::from_utf8_lossy(&payload[..])));
                    },
                    OPCODE::CONNECTION_CLOSE => {
                        /* Websocket Connection Closed */
                        debug::l1(format!("Websocket Connection Closed"));
                    }
                    _ => {
                        debug::l1(format!("Invalid OPCODE: {}", opcode));
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