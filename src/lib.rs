use errors::{Error, Result};
use serde::{Deserialize, Serialize};

pub mod client;
mod errors;
pub mod server;

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
pub struct Data<'a> {
    content: Option<&'a str>,
    source_addr: &'a str,
    target_addr: &'a str,
}

impl<'a> Data<'a> {
    pub fn new(content: Option<&'a str>, source_addr: &'a str, target_addr: &'a str) -> Self {
        Data {
            content,
            source_addr,
            target_addr,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TcpPacket<'a> {
    pub version: u8,
    pub packet_type: PacketType,
    pub length: u8,
    #[serde(borrow)]
    pub data: Option<Data<'a>>,
}

impl<'a> Default for TcpPacket<'a> {
    fn default() -> Self {
        TcpPacket {
            version: 1,
            packet_type: PacketType::Dummy,
            length: 0,
            data: None,
        }
    }
}

impl<'a> TcpPacket<'a> {
    pub fn echo_packet(v: u8) -> Self {
        TcpPacket {
            version: v,
            packet_type: PacketType::Echo,
            length: 0,
            data: None,
        }
    }

    pub fn with_data(v: u8, data: Option<Data<'a>>) -> Result<Self> {
        assert!(data.is_some(), "Data is cannot be None");

        let data_slice: Vec<u8> = bincode::serialize(&data)
            .map_err(|_| Error::Custom("Could not serialize data".to_string()))?;

        Ok(TcpPacket {
            version: v,
            packet_type: PacketType::Data,
            length: data_slice.len() as u8,
            data,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum PacketType {
    Echo,
    Data,
    Disconnect,
    Dummy,
}

#[derive(Debug, Clone, Copy)]
pub enum Status<'a> {
    AwaitingResponse(PacketType), //Maybe add timer
    SendData(PacketType, Option<Data<'a>>),
    Disconnect,
}

#[cfg(test)]
pub mod tests {
    #[test]
    fn test_server() {
        assert_eq!(1, 1)
    }
}
