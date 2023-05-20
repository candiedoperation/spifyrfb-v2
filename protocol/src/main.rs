use std::env;
use std::error::Error;
use spifyrfb_protocol::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("{}", info::license());
    println!("Version: {}, OS: {}", info::srv_version(), env::consts::OS);

    let mut launch_ip: String = String::from("");
    for arg in env::args_os() {
        if arg.to_string_lossy().starts_with("--ip=") {
            launch_ip = String::from(arg.to_string_lossy().replace("--ip=", "").trim());
        }
    }

    /* CREATE PROTOCOL SERVER WITH LAUNCH IP */
    spifyrfb_protocol::server::create(launch_ip).await.unwrap_or({});
    Ok(())
}