use serde::{Deserialize, Serialize};
use std::io::Read;
use std::time::Duration;
use std::{io, thread};
use std::{io::Write, net::TcpStream, sync::mpsc};

use anyhow::Result;

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
struct TcpPacket {
    version: u8,
    command: char,
    length: u8,
    data: Data,
}

pub struct Client {
    destination_addr: String,
    stream: Option<TcpStream>,
    sender: mpsc::Sender<String>,
    receiver: Option<mpsc::Receiver<String>>,
}

impl Client {
    pub fn new(addr: String) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            destination_addr: addr,
            stream: None,
            sender: tx,
            receiver: Some(rx),
        }
    }

    pub fn connect_to_server(&mut self) -> Result<()> {
        let server_addr = self.destination_addr.clone();

        println!("B");
        let tx = self.sender.clone();
        let stream = TcpStream::connect(server_addr)?;

        println!("C");

        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();

        self.stream = Some(stream);

        if let Some(rx) = self.receiver.take() {
            thread::spawn(move || {
                for msg in rx {
                    println!("Server Data: {msg}");
                }
            });
        }

        println!("Before loop");

        self.client_loop(tx);

        Ok(())
    }

    fn client_loop(&mut self, tx: mpsc::Sender<String>) {
        let mut buf = [0; 1024];
        if let Some(ref mut stream) = self.stream {
            loop {
                let msg = String::from("Hi from client");
                let source_addr = stream.local_addr().unwrap().to_string();
                let target_addr = stream.peer_addr().unwrap().to_string();

                let data = Data::new(Some(msg), source_addr, target_addr);
                let length = bincode::serialize(&data).unwrap().len() as u8;

                let version = 1;
                let command = 'e';

                let packet = TcpPacket {
                    version,
                    command,
                    length,
                    data,
                };

                let buf = bincode::serialize(&packet).unwrap();

                match stream.write(&buf) {
                    Ok(0) => {
                        tx.send("Server Disconnected".to_string()).unwrap();
                    }
                    Ok(n) => {
                        println!("Packet sent to server {n} bytes");
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        let echo = "Echo server".as_bytes();

                        stream.write_all(echo).expect("Could not send Echo Data");
                    }
                    Err(e) => {
                        println!("Error at reading from server {e}");
                    }
                }

                stream.flush().unwrap();

                thread::sleep(Duration::from_secs(5));
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    #[test]
    fn test_server() {
        assert_eq!(1, 1)
    }
}
