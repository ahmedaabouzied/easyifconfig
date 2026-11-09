use core::error;
use std::process::Command;

pub fn get_ifconfig() -> Result<String, Box<dyn error::Error>> {
    let output = Command::new("ifconfig").output()?;
    let output = String::from_utf8(output.stdout)?;
    Ok(output)
}
