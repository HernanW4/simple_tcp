use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread,
};

use bincode::deserialize;

use crate::{
    client::Client,
    errors::{Error, Result},
    Command, Status,
};
use crate::{Data, TcpPacket};

pub struct Server {
    listen_addr: String,
    ln: Option<TcpListener>,
    client_pool: Arc<Mutex<Vec<(TcpStream, Status)>>>,
}

//
//struct ClientInfo{
//    client_stream: TcpStream,
//    channel
//}
//

impl Server {
    pub fn new(addr: String) -> Result<Self> {
        if !addr.contains(":") {
            return Err(Error::InvalidAddress(addr));
        }

        log::info!("Server has been initialized at: {addr}");

        Ok(Server {
            listen_addr: addr,
            ln: None,
            client_pool: Arc::new(Mutex::new(Vec::with_capacity(4))),
        })
    }

    pub fn start_server(&mut self) -> Result<()> {
        let listener = TcpListener::bind(self.listen_addr.clone())?;

        self.ln = Some(listener);

        log::info!("Server has started and is reading connections");

        self.accept_loop().expect("Error at accepting connections");

        Ok(())
    }

    pub fn accept_loop(&mut self) -> Result<()> {
        if let Some(listener) = &self.ln {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let pool = Arc::clone(&self.client_pool);
                        let mut client_pool = pool.lock().unwrap();
                        log::debug!(
                            "New connection established from {}",
                            stream.peer_addr().unwrap()
                        );

                        let (tx, rx) = mpsc::channel();
                        let read_stream = Arc::new(Mutex::new(stream));
                        let write_stream = Arc::clone(&read_stream);

                        //TODO
                        //Handle clients
                        //for (client, status) in self.client_pool{
                        //    thread::spawn(move ||{
                        //        match status{
                        //            AwaitingResponse(response) => {
                        //            },
                        //            AwaitingEcho => {},
                        //            SendData(msg) => {},
                        //

                        //        }

                        //    });
                        //}
                        //match State{
                        //    Read => {},
                        //    Read if 5 secs passed => {
                        //        Write(Echo)
                        //    }
                        //    Write(Echo) => {

                        //    }
                        //    _ => {Write("Hello")}
                        //}
                        thread::spawn(move || {
                            Self::handle_read(read_stream, tx);
                        });
                        thread::spawn(move || {
                            Self::handle_write(write_stream, rx);
                        });
                    }
                    Err(e) => {
                        eprintln!("Error at getting stream: {e}");
                    }
                }
            }
        } else {
            return Err(Error::Custom("Listener is None".to_string()));
        }

        Ok(())
    }
    fn handle_write(stream: Arc<Mutex<TcpStream>>, rx: Receiver<TcpPacket>) {
        //TODO: No unwrap
        loop {
            let packet = rx.recv().unwrap();
            let data = packet.data;

            let packet_bytes = bincode::serialize(&data).unwrap();
            let length = packet_bytes.len() as u8;
            let version = 1;
            let command = Command::Echo;

            let packet = TcpPacket {
                version,
                command,
                length,
                data,
            };

            let packet_bytes = bincode::serialize(&packet).unwrap();

            let mut stream = stream.lock().unwrap();

            match stream.write(&packet_bytes) {
                Ok(0) => {
                    log::debug!("Client disconnected");
                }
                Ok(n) => {
                    log::debug!("Wrote {n} bytes");
                }
                Err(e) => {
                    log::debug!("Something went wrong here {e}");
                }
            }
            stream.flush().unwrap();
        }
    }

    fn handle_read(stream: Arc<Mutex<TcpStream>>, tx: mpsc::Sender<TcpPacket>) {
        let mut buf = [0; 1024];
        loop {
            let mut bytes_read = 0;
            let mut stream = stream.lock().unwrap();
            let client_addr = stream.peer_addr().unwrap();

            //TODO: Make it so client can say by
            match stream.read(&mut buf) {
                Ok(0) => {
                    log::debug!("Client: {client_addr} has disconnected!");
                    return;
                }
                Ok(n) => {
                    let echo_msg = format!("Echo: {}", client_addr.to_string());
                    log::debug!("Client sent a packet with {n} bytes");
                    bytes_read = n;

                    //Echo Data to client
                    stream.write_all(echo_msg.as_bytes()).unwrap();
                }

                Err(e) => {
                    eprintln!("Error reading from client {e}");
                }
            }
            //let client_msg: String = String::from_utf8_lossy(&buf[..n]).to_string();
            //
            //
            let packet: TcpPacket = deserialize(&buf[..bytes_read]).unwrap();

            log::debug!("Version received: {}", packet.version);
            assert!(packet.version == 1, "Wrong version of packet");
            assert!(
                packet.length as usize == bytes_read - 3,
                "Length of data does not match of packet signature"
            );

            //tx.send(packet).unwrap();
            //
            println!("{:?}", packet);

            stream.flush().unwrap();
        }
    }
}
