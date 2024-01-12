use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddrV4, TcpStream},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use crate::tracker::{Peer, TrackerResponse};

/// Metainfo files (also known as .torrent files).
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Torrent {
    /// The URL of the tracker.
    pub announce: String,
    /// Info This maps to a dictionary.
    pub info: Info,
}

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

    pub async fn peer_handshake(&self, peer_addr: SocketAddrV4) -> anyhow::Result<String> {
        let mut stream = TcpStream::connect(peer_addr)?;

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

        // peer id (20 bytes) (you can use 00112233445566778899 for this challenge)
        for byte in b"00112233445566778899" {
            message.push(*byte);
        }

        eprintln!(
            "sent {:?} of length {}, to {peer_addr}",
            &message,
            message.len()
        );

        stream.write_all(message.as_slice())?;

        let mut buffer = [0u8; 68];
        stream.read_exact(&mut buffer)?;
        Ok(hex::encode(&buffer[48..]))
    }
}
