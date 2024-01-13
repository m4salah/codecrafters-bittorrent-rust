use std::{
    io::{Read, Write},
    net::TcpStream,
};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PeerMessage {
    pub tag: MessageTag,
    pub payload: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageTag {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
}

impl TryFrom<u8> for MessageTag {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        return match value {
            0 => Ok(MessageTag::Choke),
            1 => Ok(MessageTag::Unchoke),
            2 => Ok(MessageTag::Interested),
            3 => Ok(MessageTag::NotInterested),
            4 => Ok(MessageTag::Have),
            5 => Ok(MessageTag::Bitfield),
            6 => Ok(MessageTag::Request),
            7 => Ok(MessageTag::Piece),
            8 => Ok(MessageTag::Cancel),
            _ => Err("invalid tag".to_string()),
        };
    }
}

#[allow(dead_code)]
impl PeerMessage {
    pub fn as_bytes(&self) -> Vec<u8> {
        // length of the payload
        let length = 1 + self.payload.len();

        let mut message = Vec::with_capacity(
            /* message tag */ 1 + /* prefix message length */ 4 + self.payload.len(),
        );

        // Length prefix (4 bytes)
        message.extend(&u32::to_be_bytes(length as u32));

        // Message identifier (1 byte)
        message.push(self.tag.clone() as u8);

        for byte in self.payload.clone() {
            message.push(byte);
        }

        // Return the constructed message
        message
    }

    pub fn from(value: &[u8]) -> Option<(Self, usize)> {
        if value.len() < 4 {
            return None;
        }

        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&value[..4]);
        let length = u32::from_be_bytes(length_bytes) as usize;

        let tag = match value[4] {
            0 => MessageTag::Choke,
            1 => MessageTag::Unchoke,
            2 => MessageTag::Interested,
            3 => MessageTag::NotInterested,
            4 => MessageTag::Have,
            5 => MessageTag::Bitfield,
            6 => MessageTag::Request,
            7 => MessageTag::Piece,
            8 => MessageTag::Cancel,
            _ => panic!("invalid tag"),
        };

        let payload = value[5..].to_vec();
        Some((Self { payload, tag }, length))
    }

    pub fn send_request_piece(stream: &mut TcpStream, index: u32, begin: u32, length: u32) {
        let mut buf = [0u8; 17];

        buf[0..4].copy_from_slice(&(1 + 3 * 4u32).to_be_bytes()); // message length
        buf[4] = 6; // message id 6 = request
        buf[5..9].copy_from_slice(&index.to_be_bytes()); // index
        buf[9..13].copy_from_slice(&begin.to_be_bytes()); // begin
        buf[13..17].copy_from_slice(&length.to_be_bytes()); // length

        stream.write_all(&buf).unwrap();
    }

    pub fn read_message(stream: &mut TcpStream) -> Self {
        let mut message_size: [u8; 4] = [0u8; 4];
        stream.read_exact(&mut message_size).unwrap();

        let message_size = u32::from_be_bytes(message_size);

        let mut buf = vec![0; message_size as usize];
        stream.read_exact(&mut buf).unwrap();

        let message_id: MessageTag = buf[0].try_into().unwrap();

        match message_id {
            MessageTag::Bitfield => Self {
                payload: Vec::new(),
                tag: MessageTag::Bitfield,
            },

            MessageTag::Unchoke => Self {
                payload: Vec::new(),
                tag: MessageTag::Unchoke,
            },
            MessageTag::Piece => {
                let mut index = [0u8; 4];

                index.copy_from_slice(&buf[1..5]);

                let mut begin = [0u8; 4];

                begin.copy_from_slice(&buf[5..9]);

                Self {
                    payload: (&buf[9..]).to_vec(),
                    tag: MessageTag::Piece,
                }
            }

            _ => panic!("Unexpected message"),
        }
    }
}
