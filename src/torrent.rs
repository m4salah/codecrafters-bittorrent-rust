use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddrV4, TcpStream},
    path::PathBuf,
};

use anyhow::Ok;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use crate::{
    peer_message::{MessageTag, PeerMessage},
    tracker::{Peer, TrackerResponse},
};

/// Metainfo files (also known as .torrent files).
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Torrent {
    /// The URL of the tracker.
    pub announce: String,
    /// Info This maps to a dictionary.
    pub info: Info,
}

const PEER_ID: &[u8; 20] = b"00112233445566778899";
#[allow(dead_code)]
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Info {
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
    pub fn new(path: PathBuf) -> Result<Torrent, anyhow::Error> {
        let torrent_byte = fs::read(path)?;
        let decoded: Torrent = serde_bencode::from_bytes(&torrent_byte)?;
        Ok(decoded)
    }

    pub fn info_hash_hex(&self) -> Result<String, anyhow::Error> {
        let bytes = serde_bencode::to_bytes(&self.info)?;
        let mut hasher = <Sha1 as Digest>::new();
        hasher.update(bytes);
        let hash = hasher.finalize();
        let hex = hex::encode(hash);
        Ok(hex)
    }

    pub fn info_hash_bytes(&self) -> [u8; 20] {
        let bytes = serde_bencode::to_bytes(&self.info).expect("it must be valid bytes");
        let mut hasher = <Sha1 as Digest>::new();
        hasher.update(&bytes);
        hasher
            .finalize()
            .try_into()
            .expect("GenericArray<_, 20> == [_; 20]")
    }

    pub fn info_hash_urlencoded(&self) -> Result<String, anyhow::Error> {
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

    pub async fn discover_peers(&self) -> Result<Vec<Peer>, anyhow::Error> {
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

    fn make_handshake(
        &self,
        stream: &mut TcpStream,
        peer_addr: SocketAddrV4,
        peer_id: [u8; 20],
    ) -> anyhow::Result<()> {
        eprintln!("connected to {peer_addr}");
        let mut message = Vec::with_capacity(68);

        // length of the protocol string (BitTorrent protocol) which is 19 (1 byte)
        message.push(19);

        // the string BitTorrent protocol (19 bytes)
        for byte in b"BitTorrent protocol" {
            message.push(*byte);
        }

        // eight reserved bytes, which are all set to zero (8 bytes)
        for byte in [0u8; 8] {
            message.push(byte);
        }

        // sha1 infohash (20 bytes) (NOT the hexadecimal representation, which is 40 bytes long)
        for byte in self.info_hash_bytes() {
            message.push(byte);
        }

        // peer id (20 bytes)
        for byte in peer_id {
            message.push(byte);
        }

        stream.write_all(message.as_slice())?;
        Ok(())
    }

    pub fn peer_handshake(&self, peer_addr: SocketAddrV4) -> anyhow::Result<String> {
        let mut stream = TcpStream::connect(peer_addr)?;
        self.make_handshake(&mut stream, peer_addr, *PEER_ID)?;
        let mut buffer = [0u8; 68];
        stream.read_exact(&mut buffer)?;
        Ok(hex::encode(&buffer[48..]))
    }

    pub async fn download_piece(&self, piece_index: u32) -> anyhow::Result<Vec<u8>> {
        // retrieve random peer to make a handshake with
        // TODO: for now there is not rand crate so i will get the first peer.
        let peers = self.discover_peers().await?;
        let peer = peers.first().expect("there is no peer");

        let mut stream = TcpStream::connect(peer.addr())?;

        // make handshake and receive the first message
        self.make_handshake(&mut stream, peer.addr(), *PEER_ID)?;
        let mut buffer = [0u8; 68];
        stream.read_exact(&mut buffer)?;

        PeerMessage::read_message(&mut stream); // Read bitfield message

        // send interest message
        let message = PeerMessage {
            payload: Vec::new(),
            tag: MessageTag::Interested,
        }
        .as_bytes();
        stream.write_all(&message)?;

        let mut piece_data = Vec::new();

        let mut block_index: u32 = 0;
        let mut block_length: u32 = 16 * 1024;

        let mut remaining_bytes = if piece_index < (self.info.pieces.len() / 20 - 1) as u32 {
            // a piece hash is 20 bytes in length
            self.info.piece_length
        } else {
            let last_len = self.info.length % self.info.piece_length;

            if last_len == 0 {
                self.info.piece_length
            } else {
                last_len
            }
        };

        while remaining_bytes != 0 {
            eprintln!("{}, {}, {}", remaining_bytes, block_index, block_length);
            if remaining_bytes < block_length as usize {
                block_length = remaining_bytes as u32;
            }

            // send request message
            PeerMessage::send_request_piece(
                &mut stream,
                piece_index as u32,
                block_index * (16 * 1024),
                block_length,
            );

            let read_incoming_message = PeerMessage::read_message(&mut stream);
            if read_incoming_message.tag == MessageTag::Piece {
                piece_data.extend(read_incoming_message.payload);
            }

            remaining_bytes -= block_length as usize;
            block_index += 1;
        }

        Ok(piece_data)
    }
}
