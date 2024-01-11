use std::{fs, path::PathBuf};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use sha1::{Digest, Sha1};

use crate::bendecoder::decode_bencoded_value;
mod bendecoder;

// Usage: your_bittorrent.sh decode "<encoded_value>"
// Usage: your_bittorrent.sh info "<file>.torrent"
/// Simple program to greet a person
#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand, Debug)]
enum Commands {
    Decode { encoded_bencode: String },
    Info { torrent: PathBuf },
}

/// Metainfo files (also known as .torrent files).
#[derive(Deserialize, Serialize, Debug)]
struct TorrentInfo {
    /// The URL of the tracker.
    pub announce: String,
    /// Info This maps to a dictionary.
    pub info: Info,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize, Debug)]
struct Info {
    /// The name key is a UTF-8 encoded string which is the suggested name to save the file (or directory) as.
    /// It is purely advisory.
    pub name: String,

    /// length - The length of the file, in bytes.
    pub length: usize,

    /// piece length is the number of bytes in each piece the file is split into.
    /// For the purposes of transfer, files are split into fixed-size pieces which are all the same length
    /// except for possibly the last one which may be truncated. piece length is almost always a power of two, most commonly 2^18 = 256K
    /// (BitTorrent prior to version 3.2 uses 2^20 = 1M as default).
    #[serde(rename = "piece length")]
    pub piece_length: usize,

    /// pieces is a string whose length is a multiple of 20.
    /// It is to be subdivided into strings of length 20,
    /// each of which is the SHA1 hash of the piece at the corresponding index.
    pub pieces: ByteBuf,
}

fn parse_torrent(torrent_path: PathBuf) -> Result<(TorrentInfo, String), anyhow::Error> {
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
    let args = Args::parse();
    match args.command {
        Commands::Decode { encoded_bencode } => {
            eprintln!("Logs from your program will appear here!");

            let decoded_value = decode_bencoded_value(&encoded_bencode);
            println!("{}", decoded_value.0);
        }
        Commands::Info { torrent } => {
            let decoded_value = parse_torrent(torrent).unwrap();
            println!("Tracker URL: {}", decoded_value.0.announce);
            println!("Length: {}", decoded_value.0.info.length);
            println!("Info Hash: {}", decoded_value.1);
            println!("Piece Length: {}", decoded_value.0.info.piece_length);
            println!("Piece Hashes: ");
            for piece in decoded_value.0.info.pieces.chunks(20) {
                let hexed = hex::encode(piece);
                println!("{}", hexed);
            }
        }
    }
}
