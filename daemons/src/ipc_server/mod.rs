pub mod event;

use std::{error::Error, time::Duration, collections::HashMap};
use once_cell::sync::Lazy;
use tokio::{net::{TcpListener, TcpStream}, io::{self, AsyncReadExt, AsyncWriteExt}, sync::RwLock, time::timeout};

pub struct IpcEvent;
impl IpcEvent {
    pub(crate) const HELLO: u8 = 0;
    pub(crate) const PING: u8 = 1;
    pub(crate) const PONG: u8 = 2;
    pub(crate) const IP_UPDATE: u8 = 3;
    pub(crate) const DISCONNECT: u8 = 4;
}

static PENDING_WRITES: Lazy<RwLock<HashMap<String, Vec<Vec<u8>>>>>
    = Lazy::new(|| { RwLock::new(HashMap::new()) } );

fn construct_payload(opcode: u8, payload: &str) -> Vec<u8> {
    let payload = payload.as_bytes();
    let mut packet: Vec<u8> = vec![];
    packet.push(opcode);
    packet.push(payload.len() as u8);
    packet.extend_from_slice(payload);
    return packet;
}

async fn push_pending_writes(endpoint: String, write: Vec<u8>) {
    let mut pendingwrites_lock = PENDING_WRITES.write().await;
    let mut updated_writes = pendingwrites_lock.get(&endpoint).unwrap().clone();
    updated_writes.push(write);

    /* Update HashMap */
    pendingwrites_lock.insert(endpoint, updated_writes);
}

async fn set_pending_writes(endpoint: String, write: Vec<Vec<u8>>) {
    let mut pendingwrites_lock = PENDING_WRITES.write().await;
    pendingwrites_lock.insert(endpoint, write);
}

async fn get_pending_writes(endpoint: String) -> Vec<Vec<u8>> {
    let pendingwrites_lock = PENDING_WRITES.read().await;
    let vector = pendingwrites_lock.get(&endpoint);
    if vector.is_some() {
        vector.unwrap().clone()
    } else {
        vec![]
    }
}

async fn init_pending_writes(endpoint: String) {
    let mut pendingwrites_lock = PENDING_WRITES.write().await;
    pendingwrites_lock.insert(endpoint, vec![]);
}

pub async fn send_ping(endpoint: String) {
    push_pending_writes(endpoint, construct_payload(IpcEvent::PING, "PING")).await;
}

async fn handle_client(client: TcpStream) {
    /* Define Function Objects */
    let mut client_pid: Option<u32> = Option::None;
    let tcp_endpoint = client.peer_addr().unwrap().to_string();
    init_pending_writes(tcp_endpoint.clone()).await;
    let (mut client_rx, mut client_tx) = io::split(client);

    /* Read and Write Concurrently (almost) */
    loop {
        let mut opcode: [u8; 2] = [0; 2];
        let rx_timeout = timeout(
            Duration::from_millis(50), 
            client_rx.read_exact(&mut opcode)
        ).await;

        if rx_timeout.is_ok() {
            let rx = rx_timeout.unwrap();
            if rx.unwrap_or(0) != 0 {
                /* opcode: Byte1 (Opcode), Byte2 (Payload Length) */
                let mut payload: Vec<u8> = vec![0; opcode[1] as usize];
                client_rx.read_exact(&mut payload).await.unwrap();     

                /* Match Opcode */
                match opcode[0] {
                    IpcEvent::HELLO => {
                        let parsed_payload = String::from_utf8_lossy(&payload).to_string();
                        let parsed_payload: Vec<&str> = parsed_payload.split("\r\n").collect();
                        client_pid = Option::Some(parsed_payload[0].parse().unwrap());

                        event::fire(
                            IpcEvent::HELLO, 
                            String::from_utf8_lossy(&payload).to_string()
                        ).await;
                    },
                    IpcEvent::PING => {
                        /* Write PONG Message */
                        push_pending_writes(
                            tcp_endpoint.clone(), 
                            construct_payload(IpcEvent::PONG, "PONG")
                        ).await;
                    },
                    IpcEvent::IP_UPDATE => {
                        push_pending_writes(tcp_endpoint.clone(), construct_payload(
                            IpcEvent::PING, 
                            &format!("Received IP Update: {}", String::from_utf8_lossy(&payload))
                        )).await;
                    }
                    _ => { /* OPCODE Invalid */ }
                }    
            } else {
                /* Server Disconnected, Fire Event */
                event::fire(
                    IpcEvent::DISCONNECT, 
                    client_pid.unwrap().to_string()
                ).await;

                /* Close the Thread */
                break;
            }       
        } else {
            /* Loop and Write */
            for payload in get_pending_writes(tcp_endpoint.clone()).await {
                client_tx
                .write_all(&payload)
                .await
                .unwrap();
            }

            /* Clear Pending Writes */
            set_pending_writes(tcp_endpoint.clone(), vec![]).await;
        }
    }
}

pub async fn create(ip_address: String) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(ip_address).await;
    if listener.is_ok() {
        let listener = listener.unwrap();
        loop {
            let (client, _) = listener.accept().await?;
            tokio::spawn(async move {
                /* Handle Client */
                handle_client(client).await;
            });
        }
    } else {
        let err = listener.err().unwrap();
        Err(err.into())
    }
}