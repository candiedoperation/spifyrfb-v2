use std::env;

fn main() {
    match env::consts::OS {
        "windows" => {
            spifyrfb_daemon::windows::create();
        }
        os => {
            println!("OS Not Supported: {}", os);
        }
    }
}