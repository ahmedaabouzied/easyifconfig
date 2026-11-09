use std::error;
use std::process;
use std::str::FromStr;

use crate::block::Iface;
use crate::render::render;

mod block;
mod ifconfig;
mod knowledge;
mod render;

fn err_exit(e: Box<dyn error::Error>) -> ! {
    eprintln!("{e}");
    process::exit(1);
}

fn main() {
    let content = match ifconfig::get_ifconfig() {
        Ok(c) => c,
        Err(e) => err_exit(e),
    };
    let blocks = match parse_blocks(&content) {
        Ok(b) => b,
        Err(e) => err_exit(e),
    };

    println!("{}", render(&blocks, true));
}

fn raw_blocks(content: &str) -> Vec<String> {
    let mut current: Option<String> = None;
    let mut result = Vec::new();

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with(char::is_whitespace) {
            // detail line: keep it only if we're inside a real block
            if let Some(block) = current.as_mut() {
                block.push_str(line);
                block.push('\n');
            }
        } else {
            // column 0: close the open block, then open a new one if this is a header
            result.extend(current.take());
            current = line.contains(": flags=").then(|| format!("{line}\n"));
        }
    }
    result.extend(current);
    result
}
fn parse_blocks(content: &str) -> Result<Vec<Iface>, Box<dyn error::Error>> {
    let mut res = Vec::<Iface>::new();
    for raw_block in raw_blocks(content) {
        let parsed_block = block::Iface::from_str(&raw_block)?;
        res.push(parsed_block);
    }
    Ok(res)
}

#[cfg(test)]
mod test {
    use std::{fs::OpenOptions, io::Read, path::PathBuf};

    use crate::parse_blocks;

    #[test]
    fn parse_sample_file() {
        let mac_sample = PathBuf::from("./samples/mac.txt");
        let mut sample_file = OpenOptions::new().read(true).open(mac_sample).unwrap();
        let mut content = String::new();
        sample_file.read_to_string(&mut content).unwrap();
        let blocks = parse_blocks(&content).unwrap();
        blocks.iter().for_each(|b| println!("{b}"));
    }
}
