use std::{fs, path::PathBuf};

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
}
