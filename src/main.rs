use std::env;
use std::error::Error;
use spifyrfb::info;
use spifyrfb::server;
use spifyrfb::win32;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("{}", info::license());
    println!("Version: {}, OS: {}", info::srv_version(), env::consts::OS);
    
    //server::create(String::from("192.168.56.101:8080")).await?;
    win32::service::create();
    Ok(())
}