use std::convert::TryFrom;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use bytes::{Bytes, BytesMut, BufMut};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgType {
    Auth = 0x01,
    AuthOk = 0x02,
    AuthFail = 0x03,
    Frame = 0x10,
    MouseEvent = 0x20,
    KeyEvent = 0x21,
    Clipboard = 0x30,
    Audio = 0x40,
    Disconnect = 0xFF,
}

impl TryFrom<u8> for MsgType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(MsgType::Auth),
            0x02 => Ok(MsgType::AuthOk),
            0x03 => Ok(MsgType::AuthFail),
            0x10 => Ok(MsgType::Frame),
            0x20 => Ok(MsgType::MouseEvent),
            0x21 => Ok(MsgType::KeyEvent),
            0x30 => Ok(MsgType::Clipboard),
            0x40 => Ok(MsgType::Audio),
            0xFF => Ok(MsgType::Disconnect),
            _ => Err(()),
        }
    }
}

pub async fn send_message(stream: &mut TcpStream, msg_type: MsgType, payload: &[u8]) -> io::Result<()> {
    let payload_len = payload.len() as u32;
    let mut header = [0u8; 5];
    header[0..4].copy_from_slice(&payload_len.to_be_bytes());
    header[4] = msg_type as u8;

    stream.write_all(&header).await?;
    if payload_len > 0 {
        stream.write_all(payload).await?;
    }
    stream.flush().await?;
    Ok(())
}

pub async fn recv_message(stream: &mut TcpStream) -> io::Result<Option<(MsgType, Bytes)>> {
    let mut header = [0u8; 5];
    match stream.read_exact(&mut header).await {
        Ok(_) => {}
        Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }

    let length = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
    let msg_type_val = header[4];

    let msg_type = match MsgType::try_from(msg_type_val) {
        Ok(t) => t,
        Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid message type")),
    };

    if length > 0 {
        let mut payload = BytesMut::with_capacity(length);
        payload.put_bytes(0, length);
        stream.read_exact(payload.as_mut()).await?;
        Ok(Some((msg_type, payload.freeze())))
    } else {
        Ok(Some((msg_type, Bytes::new())))
    }
}
