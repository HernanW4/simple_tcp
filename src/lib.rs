use serde::{Deserialize, Serialize};

pub mod client;
mod errors;
pub mod server;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Data {
    content: Option<String>,
    source_addr: String,
    target_addr: String,
}

impl Data {
    pub fn new(content: Option<String>, source_addr: String, target_addr: String) -> Self {
        Data {
            content,
            source_addr,
            target_addr,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TcpPacket {
    version: u8,
    command: Command,
    length: u8,
    data: Option<Data>,
}

impl TcpPacket {
    pub fn echo_packet(v: u8) -> Self {
        TcpPacket {
            version: v,
            command: Command::Echo,
            length: 0,
            data: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Echo,
    Data,
    Disconnect,
}

#[derive(Debug)]
pub enum Status {
    Write(TcpPacket),
    AwaitingResponse, //Maybe add timer
    AwaitingEcho,
    WriteEcho,
}

#[cfg(test)]
pub mod tests {
    #[test]
    fn test_server() {
        assert_eq!(1, 1)
    }
}
