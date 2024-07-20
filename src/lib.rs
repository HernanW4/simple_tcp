use serde::{Deserialize, Serialize};

pub mod client;
mod errors;
pub mod server;
mod utils;

pub const PACKET_VERSION: u8 = 1;

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
pub struct Data<'a> {
    content: Option<&'a str>,
}

impl<'a> Data<'a> {
    pub fn new(content: Option<&'a str>) -> Self {
        Data { content }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TcpPacket<'a> {
    pub version: u8,
    pub packet_type: PacketType,
    pub seq_num: u32,
    pub ack_num: u32,
    #[serde(borrow)]
    pub data: Option<Data<'a>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    Syn,
    Ack,
    SynAck,
    Data,
    Fin,
}

#[derive(Debug, Clone, Copy)]
pub enum ConnectionState {
    Closed,
    Established,
    Listen,
    SynSent,
    SynRecv,
    CloseWait,
    Closing,
}

#[cfg(test)]
pub mod tests {
    #[test]
    fn test_server() {
        assert_eq!(1, 1)
    }
}
