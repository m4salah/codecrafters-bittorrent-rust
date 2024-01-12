use std::{
    fs,
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
};

use anyhow::Ok;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
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
    Peers { torrent: PathBuf },
}

/// Metainfo files (also known as .torrent files).
#[derive(Clone, Deserialize, Serialize, Debug)]
struct Torrent {
    /// The URL of the tracker.
    pub announce: String,
    /// Info This maps to a dictionary.
    pub info: Info,
}

#[allow(dead_code)]
#[derive(Clone, Deserialize, Serialize, Debug)]
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
    #[serde(with = "serde_bytes")]
    pub pieces: Vec<u8>,
}

impl Torrent {
    fn info_hash_hex(&self) -> Result<String, anyhow::Error> {
        let bytes = serde_bencode::to_bytes(&self.info)?;
        let mut hasher = <Sha1 as Digest>::new();
        hasher.update(bytes);
        let hash = hasher.finalize();
        let hex = hex::encode(hash);
        Ok(hex)
    }
    fn info_hash_urlencoded(&self) -> Result<String, anyhow::Error> {
        let hash = self.info_hash_hex()?;

        let mut urlencoded = String::new();

        for (i, char) in hash.chars().enumerate() {
            if i % 2 == 0 {
                urlencoded.push('%');
            }
            urlencoded.push(char);
        }
        Ok(urlencoded)
    }

    fn new(path: PathBuf) -> Result<Torrent, anyhow::Error> {
        let torrent_byte = fs::read(path)?;
        let decoded: Torrent = serde_bencode::from_bytes(&torrent_byte)?;
        Ok(decoded)
    }

    async fn discover_peers(&self) -> Result<Vec<Peer>, anyhow::Error> {
        let endpoint = format!(
            "{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact={}",
            self.announce,
            self.info_hash_urlencoded().unwrap(),
            "00112233445566778899",
            6881,
            0,
            0,
            self.info.length,
            1
        );
        let response = reqwest::get(endpoint).await?.bytes().await?;
        let decoded: TrackerResponse = serde_bencode::from_bytes(&response)?;

        Ok(decoded.all_peers())
    }
}

#[allow(dead_code)]
#[derive(Clone, Deserialize, Debug)]
struct TrackerResponse {
    /// An integer, indicating how often your client should make a request to the tracker.
    interval: usize,

    /// A string, which contains list of peers that your client can connect to.
    /// Each peer is represented using 6 bytes.
    /// The first 4 bytes are the peer's IP address and the last 2 bytes are the peer's port number.
    #[serde(with = "serde_bytes")]
    peers: Vec<u8>,
}

impl TrackerResponse {
    fn all_peers(&self) -> Vec<Peer> {
        let mut peers = Vec::new();
        for chunk_6 in self.peers.chunks(6) {
            let addr = Ipv4Addr::new(chunk_6[0], chunk_6[1], chunk_6[2], chunk_6[3]);
            let port = u16::from_be_bytes([chunk_6[4], chunk_6[5]]);
            peers.push(Peer(SocketAddrV4::new(addr, port)));
        }
        return peers;
    }
}
#[derive(Clone, Deserialize, Debug)]
struct Peer(SocketAddrV4);

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    match args.command {
        Commands::Decode { encoded_bencode } => {
            eprintln!("Logs from your program will appear here!");

            let decoded_value = decode_bencoded_value(&encoded_bencode);
            println!("{}", decoded_value.0);
        }
        Commands::Info { torrent } => {
            let decoded_value = Torrent::new(torrent).unwrap();
            println!("Tracker URL: {}", decoded_value.announce);
            println!("Length: {}", decoded_value.info.length);
            println!("Info Hash: {}", decoded_value.info_hash_hex().unwrap());
            println!("Piece Length: {}", decoded_value.info.piece_length);
            println!("Piece Hashes: ");
            for piece in decoded_value.info.pieces.chunks(20) {
                let hexed = hex::encode(piece);
                println!("{}", hexed);
            }
        }
        Commands::Peers { torrent } => {
            let decoded_value = Torrent::new(torrent).unwrap();
            let peers = decoded_value.discover_peers().await?;
            for peer in peers {
                println!("{}", peer.0);
            }
        }
    }
    Ok(())
}
