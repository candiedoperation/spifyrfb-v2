use std::env;
use std::error::Error;
use libvncrustserver::info;
use libvncrustserver::server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Version: {}, OS: {}", info::srv_version(), env::consts::OS);
    server::create(String::from("127.0.0.1:8080")).await?;
    Ok(())
}