use std::{
    env,
    fmt::{Display, Write},
    fs,
};

use serde::Deserialize;

use crate::bendecoder::decode_bencoded_value;
mod bendecoder;

// Usage: your_bittorrent.sh decode "<encoded_value>"
// Usage: your_bittorrent.sh info "<file>.torrent"

#[derive(Deserialize, Debug)]
struct TorrentInfo {
    pub announce: String,
    pub info: Info,
}

impl Display for TorrentInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("Tracker URL: {}", self.announce).as_str())?;
        f.write_char('\n')?;
        f.write_str(format!("{}", self.info).as_str())
    }
}

#[derive(Deserialize, Debug)]
struct Info {
    pub length: usize,
}

impl Display for Info {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("Length: {}", self.length).as_str())
    }
}

fn parse_torrent(torrent_path: &str) -> Result<TorrentInfo, anyhow::Error> {
    let torrent_byte = fs::read(torrent_path)?;
    let decoded: TorrentInfo = serde_bencode::from_bytes(&torrent_byte)?;

    Ok(decoded)
}
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        eprintln!("Logs from your program will appear here!");

        let encoded_value = &args[2];
        let decoded_value = decode_bencoded_value(encoded_value);
        println!("{}", decoded_value.0);
    } else if command == "info" {
        let torrent_path = &args[2];
        let decoded_value = parse_torrent(&torrent_path).unwrap();
        println!("{}", decoded_value);
    } else {
        eprintln!("unknown command: {}", args[1])
    }
}
