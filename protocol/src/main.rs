use std::env;
use std::error::Error;
use spifyrfb_protocol::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("{}", info::license());
    println!("Version: {}, OS: {}", info::srv_version(), env::consts::OS);
    
    spifyrfb_protocol::server::create(String::from("127.0.0.1:8080")).await?;
    Ok(())
}