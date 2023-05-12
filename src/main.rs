use std::error::Error;
use libvncrustserver::info;
use libvncrustserver::server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Version: {}", info::srv_version());
    server::create().await?;
    Ok(())
}