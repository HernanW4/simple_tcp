use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
};

use crate::errors::{Error, Result};
use crate::Data;

pub struct Server {
    listen_addr: String,
    ln: Option<TcpListener>,
    sender: mpsc::Sender<Data>,
    receiver: Option<mpsc::Receiver<Data>>,
}

impl Server {
    pub fn new(addr: String) -> Result<Self> {
        if !addr.contains(":") {
            return Err(Error::InvalidAddress(addr));
        }

        let (tx, rx) = mpsc::channel::<Data>();

        log::info!("Server has started at: {addr}");

        Ok(Server {
            listen_addr: addr,
            ln: None,
            sender: tx,
            receiver: Some(rx),
        })
    }

    pub fn start_server(&mut self) -> Result<()> {
        let listener = TcpListener::bind(self.listen_addr.clone())?;

        self.ln = Some(listener);

        if let Some(rx) = self.receiver.take() {
            thread::spawn(move || {
                for msg in rx {
                    let content = msg.content.unwrap_or("No content in Data".to_string());
                }
            });
        } else {
            return Err(Error::ServerReceiverNotFound);
        }

        self.accept_loop().expect("Error at accepting connections");

        Ok(())
    }

    pub fn accept_loop(&mut self) -> Result<()> {
        if let Some(listener) = &self.ln {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let tx = self.sender.clone();
                        thread::spawn(move || {
                            read_stream(stream, tx);
                        });
                    }
                    Err(e) => {
                        println!("Error at getting stream: {e}");
                    }
                }
            }
        } else {
            return Err(Error::Custom("Listener is None".to_string()));
        }

        Ok(())
    }
}
fn read_stream(mut stream: TcpStream, tx: mpsc::Sender<Data>) {
    let mut buf = [0; 1024];

    let client_addr = stream
        .peer_addr()
        .expect("Could not identify the address of client");
    loop {
        match stream.read(&mut buf) {
            Ok(n) if n <= 2 => {
                println!("Client: {client_addr} has disconnected!");
                return;
            }
            Ok(n) => {
                let client_msg: String = String::from_utf8_lossy(&buf[..n]).to_string();

                let echo_msg = format!("Echo: {client_msg}");

                let msg = Data::new(echo_msg.len() as u8, Some(client_msg));

                tx.send(msg).unwrap();

                //Echo Data to client
                stream.write_all(echo_msg.as_bytes()).unwrap();
            }

            Err(e) => {
                println!("Error reading from client {e}");
            }
        }
        stream.flush().unwrap();
    }
}
