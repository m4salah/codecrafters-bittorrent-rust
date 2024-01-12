use std::{net::SocketAddrV4, path::PathBuf};

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
    Decode {
        encoded_bencode: String,
    },
    Info {
        torrent: PathBuf,
    },
    Peers {
        torrent: PathBuf,
    },
    Handshake {
        torrent: PathBuf,
        peer_addr: SocketAddrV4,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match args.command {
        Commands::Decode { encoded_bencode } => {
            eprintln!("Logs from your program will appear here!");

            let decoded_value = decode_bencoded_value(&encoded_bencode);
            println!("{}", decoded_value.0);
        }
        Commands::Info { torrent } => {
            let torrent = Torrent::new(torrent)?;
            println!("Tracker URL: {}", torrent.announce);
            println!("Length: {}", torrent.info.length);
            println!("Info Hash: {}", torrent.info_hash_hex()?);
            println!("Piece Length: {}", torrent.info.piece_length);
            println!("Piece Hashes: ");
            for piece in torrent.info.pieces.chunks(20) {
                let hexed = hex::encode(piece);
                println!("{}", hexed);
            }
        }
        Commands::Peers { torrent } => {
            let torrent = Torrent::new(torrent)?;
            let peers = torrent.discover_peers().await?;
            for peer in peers {
                println!("{}", peer);
            }
        }
        Commands::Handshake { torrent, peer_addr } => {
            let torrent = Torrent::new(torrent)?;
            let peer_id = torrent.peer_handshake(peer_addr).await?;
            println!("Peer ID: {}", peer_id);
        }
    }
    Ok(())
}
