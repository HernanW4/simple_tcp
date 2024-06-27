use serde::{Deserialize, Serialize};
use std::io::Read;
use std::time::Duration;
use std::{io, thread};
use std::{io::Write, net::TcpStream, sync::mpsc};

use anyhow::Result;

mod errors;
pub mod server;

//My visualization of what a TCP packet should look like
//Structure of the packet sent through server
//
struct Data {
    content: Option<String>,
    length: u8,
}

impl Data {
    pub fn new(length: u8, content: Option<String>) -> Self {
        Data { length, content }
    }
}
struct TcpPacket {
    version: u8,
    command: Vec<u8>,
    length: u8,
    data: Vec<u8>,
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
                match stream.read(&mut buf) {
                    Ok(0) => {
                        tx.send("Server Disconnected".to_string()).unwrap();
                    }
                    Ok(n) => {
                        let server_msg = String::from_utf8_lossy(&buf[..n]).to_string();
                        tx.send(server_msg).unwrap();
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        let echo = "Echo server".as_bytes();

                        stream.write_all(echo).expect("Could not send Echo Data");
                    }
                    Err(e) => {
                        println!("Error at reading from server {e}");
                    }
                }

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
