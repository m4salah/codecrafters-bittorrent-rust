use std::{
    env,
    fmt::{Display, Write},
    fs,
};

use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use sha1::{Digest, Sha1};

use crate::bendecoder::decode_bencoded_value;
mod bendecoder;

// Usage: your_bittorrent.sh decode "<encoded_value>"
// Usage: your_bittorrent.sh info "<file>.torrent"

#[derive(Deserialize, Serialize, Debug)]
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

#[allow(dead_code)]
#[derive(Deserialize, Serialize, Debug)]
struct Info {
    pub length: usize,
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: usize,
    pub pieces: ByteBuf,
}

impl Display for Info {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("Length: {}", self.length).as_str())?;
        f.write_char('\n')?;
        f.write_str(format!("Name: {}", self.name).as_str())?;
        f.write_char('\n')?;
        f.write_str(format!("Piece Length: {}", self.piece_length).as_str())
    }
}

fn parse_torrent(torrent_path: &str) -> Result<(TorrentInfo, String), anyhow::Error> {
    let torrent_byte = fs::read(torrent_path)?;
    let decoded: TorrentInfo = serde_bencode::from_bytes(&torrent_byte)?;
    let bytes = serde_bencode::to_bytes(&decoded.info).unwrap();
    let mut hasher = Sha1::new();

    hasher.update(bytes);
    let hash = hasher.finalize();
    let hexed = hex::encode(hash);
    Ok((decoded, hexed))
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
        println!("{}", decoded_value.0);
        println!("Info Hash: {}", decoded_value.1);
    } else {
        eprintln!("unknown command: {}", args[1])
    }
}
