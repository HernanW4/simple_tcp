use anyhow::Result;
use std::io::Read;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::{io::Write, net::TcpStream};

use std::thread;
use std::time::Duration;

use crate::TcpPacket;

pub struct Client {
    //Change
    pub destination_addr: String,
    stream: Option<TcpStream>,
}

impl Client {
    pub fn new(addr: String) -> Self {
        Self {
            destination_addr: addr,
            stream: None,
        }
    }

    pub fn from_stream(stream: TcpStream) -> Self {
        let addr = stream.local_addr().expect("Could not get peer address");
        Self {
            stream: Some(stream),
            destination_addr: addr.to_string(),
        }
    }

    //Maybe?
    //pub fn from_stream(stream: TcpStream) -> Self {
    //    let address = stream.peer_addr().unwrap();
    //    Client {
    //        destination_addr: address,
    //        stream,
    //    }
    //}

    pub fn connect_to_server(&mut self) -> Result<()> {
        let server_addr = self.destination_addr.clone();

        println!("B");
        let stream = TcpStream::connect(server_addr)?;

        println!("C");

        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();

        self.stream = Some(stream);

        println!("Before loop");

        loop {
            self.client_loop();
        }
    }

    fn client_loop(&mut self) {
        if let Some(stream) = self.stream.take() {
            let (tx, rx) = mpsc::channel();
            let read_stream = Arc::new(Mutex::new(stream));
            let write_stream = Arc::clone(&read_stream);

            thread::spawn(move || {
                Self::read_server(read_stream, tx);
            });
            thread::spawn(move || {
                Self::write_to_server(write_stream, rx);
                thread::sleep(Duration::from_secs(1));
            });
        }
    }
    fn write_to_server(stream: Arc<Mutex<TcpStream>>, rx: Receiver<TcpPacket>) {
        let mut stream_lock: MutexGuard<TcpStream>;
        let mut packet: TcpPacket;

        loop {
            if let Ok(msg) = rx.recv() {
                stream_lock = stream.lock().unwrap();
                packet = msg;
            } else {
                break;
            }

            let packet = bincode::serialize(&packet).unwrap();

            match stream_lock.write(&packet) {
                Ok(n) => {
                    log::info!("Written to server {n} bytes");
                }
                Err(e) => {
                    eprintln!("Some unexpected error has happened {e}");
                }
            }

            stream_lock.flush().unwrap();
        }
    }
    fn read_server(stream: Arc<Mutex<TcpStream>>, tx: Sender<TcpPacket>) {
        let mut buf = [0; 1024];
        let mut packet_length = 0;
        let mut stream = stream.lock().unwrap();

        loop {
            match stream.read(&mut buf) {
                Ok(n) => {
                    packet_length = n;
                    log::info!("New message from server. {n} bytes");
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    let echo_packet = TcpPacket::echo_packet(1);
                    tx.send(echo_packet).unwrap();

                    //log::info!("Server has not sent anything in {} period");
                }

                Err(e) => {
                    eprintln!("Some unexpected error has happened {e}");
                }
            }

            let packet: TcpPacket = bincode::deserialize(&buf[..packet_length]).unwrap();

            assert!(packet.version == 1, "Packet version does not match");
            assert!(
                packet.length == (packet_length - 3) as u8,
                "Data length does not match one in TcpPacket"
            );
            tx.send(packet).unwrap();
            stream.flush().unwrap();
        }
    }
}
