use std::{fs, net::SocketAddrV4, path::PathBuf};

use clap::{Parser, Subcommand};

use crate::{bendecoder::decode_bencoded_value, torrent::Torrent};
mod bendecoder;
mod peer_message;
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
#[clap(rename_all = "snake_case")]
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
    DownloadPiece {
        #[arg(short, long)]
        output: PathBuf,
        torrent: PathBuf,
        piece: u32,
    },
    Download {
        #[arg(short, long)]
        output: PathBuf,
        torrent: PathBuf,
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
            let peer_id = torrent.peer_handshake(peer_addr)?;
            println!("Peer ID: {}", peer_id);
        }
        Commands::DownloadPiece {
            output,
            torrent,
            piece,
        } => {
            println!("{:?} {:?} {}", output, torrent, piece);
            let torrent = Torrent::new(torrent)?;
            let data = torrent.download_piece(piece).await?;
            fs::write(output, data).unwrap();
        }
        Commands::Download { output, torrent } => {
            let torrent_file = Torrent::new(torrent.clone())?;
            let data = torrent_file.download_all().await?;
            fs::write(output.clone(), data).unwrap();
            println!("Downloaded {:?} to {:?}.", torrent, output);
        }
    }
    Ok(())
}
