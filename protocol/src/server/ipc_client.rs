use std::{error::Error, time::{Duration, SystemTime}, thread, process};
use once_cell::sync::Lazy;
use tokio::{net::TcpStream, io::{self, AsyncReadExt, AsyncWriteExt}, time::timeout, sync::RwLock};

struct OPCODE;
impl OPCODE {
    const PING: u8 = 1;
    const PONG: u8 = 2;
    const IP_UPDATE: u8 = 3;
    const SHUTDOWN: u8 = 4;
}

static PENDING_WRITES: Lazy<RwLock<Vec<Vec<u8>>>> 
    = Lazy::new(|| { RwLock::new(vec![]) });

async fn push_pending_writes(write: Vec<u8>) {
    let mut pendingwrites_lock = PENDING_WRITES.write().await;
    pendingwrites_lock.push(write);
}

fn construct_payload(opcode: u8, payload: &str) -> Vec<u8> {
    let payload = payload.as_bytes();
    let mut packet: Vec<u8> = vec![];
    packet.push(opcode);
    packet.push(payload.len() as u8);
    packet.extend_from_slice(payload);
    return packet;
}

pub async fn connect(ip_address: String) -> Result<(), Box<dyn Error>> {
    let remote = TcpStream::connect(ip_address.clone()).await;
    if remote.is_ok() {
        println!("Connected to SpifyRFB Daemon at {:?}", ip_address);
        let (mut remote_rx, mut remote_tx) = io::split(remote.unwrap());

        loop {
            let mut opcode: [u8; 2] = [0; 2];
            let rx_timeout = timeout(
                Duration::from_millis(100),
                remote_rx.read_exact(&mut opcode)
            ).await;

            if rx_timeout.is_ok() {
                let rx = rx_timeout.unwrap();
                if rx.unwrap_or(0) != 0 {
                    /* opcode: Byte1 (Opcode), Byte2 (Payload Length) */
                    let mut payload: Vec<u8> = vec![0; opcode[1] as usize];
                    remote_rx.read_exact(&mut payload).await.unwrap();

                    /* Process payload based on Opcode */
                    match opcode[0] {
                        OPCODE::PING => {
                            /* Push to Pending Writes */
                            println!("SpifyRFB Daemon: {}", String::from_utf8_lossy(&payload));
                            let mut pendingwrites_lock = PENDING_WRITES.write().await;
                            pendingwrites_lock.push(construct_payload(OPCODE::PONG, "PONG"));
                        },
                        OPCODE::PONG => {
                            println!("PONG");
                        }
                        _ => { /* OPCODE Invalid */ }
                    }                       
                } else {
                    /* Daemon Disconnected, We are dependent on it for exit */
                    process::exit(0);
                }
            } else {
                let mut pendingwrites_lock = PENDING_WRITES.write().await;
                for payload in pendingwrites_lock.to_vec() {
                    remote_tx
                    .write_all(&payload)
                    .await
                    .unwrap();
                }

                /* Clear Pending Writes */
                *pendingwrites_lock = vec![];
            }
        }
    } else {
        let err = remote.err().unwrap();
        println!("SpifyRFB Daemon Connection Failed -> {}", err.to_string());
        Err(err.into())
    }
}