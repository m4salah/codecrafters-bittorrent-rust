use anyhow::bail;
use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Message {
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
impl Message {
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

    pub fn new_request(index: u32, begin: u32, length: u32) -> Self {
        let mut payload = Vec::with_capacity(12);
        payload.extend(index.to_be_bytes());
        payload.extend(begin.to_be_bytes());
        payload.extend(length.to_be_bytes());

        Self {
            tag: MessageTag::Request,
            payload,
        }
    }

    pub fn read_block(&self) -> anyhow::Result<Vec<u8>> {
        match self.tag {
            MessageTag::Piece => Ok(self.payload[9..].to_vec()),

            _ => bail!("Unexpected message"),
        }
    }
}
pub struct MessageFramer;

const MAX: usize = 1 << 16;

impl Decoder for MessageFramer {
    type Item = Message;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            // Not enough data to read length marker.
            return Ok(None);
        }

        // Read length marker.
        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let length = u32::from_be_bytes(length_bytes) as usize;

        if length == 0 {
            // this is a heartbeat message.
            // discard it.
            src.advance(4);
            // and then try again in case the buffer has more messages
            return self.decode(src);
        }

        if src.len() < 5 {
            // Not enough data to read tag marker.
            return Ok(None);
        }

        // Check that the length is not too large to avoid a denial of
        // service attack where the server runs out of memory.
        if length > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame of length {} is too large.", length),
            ));
        }

        if src.len() < 4 + length {
            // The full string has not yet arrived.
            //
            // We reserve more space in the buffer. This is not strictly
            // necessary, but is a good idea performance-wise.
            src.reserve(4 + length - src.len());

            // We inform the Framed that we need more bytes to form the next
            // frame.
            return Ok(None);
        }

        let tag = match src[4] {
            0 => MessageTag::Choke,
            1 => MessageTag::Unchoke,
            2 => MessageTag::Interested,
            3 => MessageTag::NotInterested,
            4 => MessageTag::Have,
            5 => MessageTag::Bitfield,
            6 => MessageTag::Request,
            7 => MessageTag::Piece,
            8 => MessageTag::Cancel,
            tag => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unknown message type {}.", tag),
                ))
            }
        };

        // extract the payload according to the length marker appers in the first 4 bytes of the src.
        let data = if src.len() > 5 {
            src[5..4 + length].to_vec()
        } else {
            Vec::new()
        };

        // Use advance to modify src such that it no longer contains
        // this frame.
        src.advance(4 + length);

        Ok(Some(Message { tag, payload: data }))
    }
}

impl Encoder<Message> for MessageFramer {
    type Error = std::io::Error;

    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Don't send a message if it is longer than the other end will
        // accept.
        if item.payload.len() + 1 > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame of length {} is too large.", item.payload.len()),
            ));
        }

        // Convert the length into a byte array.
        let len_slice = u32::to_be_bytes(item.payload.len() as u32 + 1 /* tag */);

        // Reserve space in the buffer.
        dst.reserve(4 /* length */ + 1 /* tag */ + item.payload.len());

        // Write the length and string to the buffer.
        dst.extend_from_slice(&len_slice);
        dst.put_u8(item.tag as u8);
        dst.extend_from_slice(&item.payload);
        Ok(())
    }
}
