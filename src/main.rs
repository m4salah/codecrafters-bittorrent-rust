use std::path::PathBuf;

use anyhow::Ok;
use clap::{Parser, Subcommand};

use crate::{bendecoder::decode_bencoded_value, torrent::Torrent};
mod bendecoder;
mod torrent;
mod tracker;

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
                println!("{}", peer);
            }
        }
    }
    Ok(())
}
