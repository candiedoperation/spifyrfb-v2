use std::env;
use std::error::Error;
use libvncrustserver::info;
use libvncrustserver::server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Version: {}, OS: {}", info::srv_version(), env::consts::OS);
    server::create(String::from("192.168.56.101:8080")).await?;
    Ok(())
}