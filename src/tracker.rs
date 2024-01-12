use std::{
    fmt::Display,
    net::{Ipv4Addr, SocketAddrV4},
};

use serde::Deserialize;

#[allow(dead_code)]
#[derive(Clone, Deserialize, Debug)]
pub struct TrackerResponse {
    /// An integer, indicating how often your client should make a request to the tracker.
    interval: usize,

    /// A string, which contains list of peers that your client can connect to.
    /// Each peer is represented using 6 bytes.
    /// The first 4 bytes are the peer's IP address and the last 2 bytes are the peer's port number.
    #[serde(with = "serde_bytes")]
    peers: Vec<u8>,
}

impl TrackerResponse {
    pub fn all_peers(&self) -> Vec<Peer> {
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
pub struct Peer(SocketAddrV4);

#[allow(dead_code)]
impl Peer {
    pub fn addr(&self) -> SocketAddrV4 {
        self.0
    }
}
impl Display for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}
