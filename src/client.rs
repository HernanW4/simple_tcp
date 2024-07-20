use std::io::{self, Read};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::{io::Write, net::TcpStream};

use std::thread;
use std::time::{Duration, Instant};

use crate::errors::{Error, Result};
use crate::{Data, PacketType, Status, TcpPacket};

const READ_TIMEOUT: Duration = Duration::from_secs(3);

pub struct Client<'a> {
    //Change
    pub destination_addr: String,
    stream: TcpStream,
    status: Status<'a>,
}

impl<'a> Client<'a> {
    pub fn new(addr: String) -> Result<Self> {
        if let Ok(stream) = TcpStream::connect(&addr) {
            stream
                .set_read_timeout(Some(READ_TIMEOUT))
                .map_err(|e| Error::IO(e))?;
            Ok(Client {
                destination_addr: addr,
                stream,
                status: Status::AwaitingResponse(PacketType::Echo),
            })
        } else {
            Err(Error::Custom(
                "Could not create a client with server address {addr}".to_string(),
            ))
        }
    }

    pub fn run(&mut self) -> Result<()> {
        println!("H");
        let mut last_activity = Instant::now();

        let mut buf: [u8; 1024] = [0; 1024];

        let timeout = Duration::from_secs(7);
        let mut tries = 0;

        //Main client loop
        loop {
            match self.status {
                Status::AwaitingResponse(PacketType::Echo) if last_activity.elapsed() > timeout => {
                    if tries < 3 {
                        log::debug!("Checking with server again!");
                        self.status = Status::SendData(PacketType::Echo, None);
                        tries += 1;
                        last_activity = Instant::now();
                    } else {
                        self.status = Status::Disconnect;
                        tries = 0;
                    }
                }
                Status::AwaitingResponse(PacketType::Echo) => {
                    let bytes_read = Self::read_from_stream(&mut self.stream, &mut buf)
                        .map_err(|_| Error::Custom("Could not read from stream".to_string()))?;

                    if bytes_read == 0 {
                        self.status = Status::Disconnect;
                    } else if bytes_read > 1 {
                        if let Ok((packet_type, data)) = Self::process_bytes(&buf) {
                            match packet_type {
                                PacketType::Echo => {
                                    self.status = Status::SendData(PacketType::Echo, None);
                                }
                                PacketType::Data => {
                                    log::debug!("I have received Data! {:?}", data);
                                }
                                PacketType::Disconnect => {
                                    log::debug!("Disconnecting from server");
                                    return Ok(());
                                }
                                _ => todo!(),
                            }

                            last_activity = Instant::now();
                            tries = 0;
                        }
                    }
                }

                Status::AwaitingResponse(PacketType::Data) => {}

                Status::SendData(PacketType::Echo, _) => {
                    let buf = Self::data_to_packet(PacketType::Echo, None).map_err(|_| {
                        Error::Custom("Could not convert data to packer".to_string())
                    })?;

                    let bytes = Self::write_to_stream(&mut self.stream, &buf)
                        .map_err(|_| Error::Custom("Error at writing to stream".to_string()))?;

                    if bytes <= 0 {
                        log::warn!(
                            "Something went wrong when writing to server. Wrote {bytes} bytes"
                        );
                    }
                    log::debug!("Succesfully sent {bytes} bytes to server!!");
                    last_activity = Instant::now();
                    self.status = Status::AwaitingResponse(PacketType::Echo);
                }

                Status::SendData(PacketType::Data, Some(data)) => {}

                Status::Disconnect => {
                    log::warn!("Disconnecting from server");
                    return Ok(());
                }

                _ => {}
            }

            self.stream.flush().expect("Could not flush stream");
            buf = [0; 1024];
        }
    }

    fn data_to_packet(packet_type: PacketType, data: Option<Data>) -> Result<Vec<u8>> {
        let data_len = bincode::serialize(&data)
            .map_err(|_| Error::Custom("Could not serialize data".to_string()))?
            .len();

        let tcp_packet = TcpPacket {
            version: 1,
            packet_type,
            length: data_len as u8,
            data,
        };

        bincode::serialize(&tcp_packet)
            .map_err(|_| Error::Custom("Couldn't serialize tcp packet".to_string()))
    }

    fn write_to_stream(stream: &mut TcpStream, buf: &[u8]) -> Result<usize> {
        let mut bytes_written = 0;

        match stream.write(buf) {
            Ok(0) => {
                log::error!("Disconnect? ");
            }
            Ok(n) => {
                log::debug!("Writtent {n} bytes to server!");
                bytes_written = n;
            }
            Err(e) => {
                log::error!("Something went wrong when writing to server{e}")
            }
        }

        stream.flush().expect("Could not flush stream");

        Ok(bytes_written)
    }

    fn read_from_stream(stream: &mut TcpStream, buf: &mut [u8]) -> Result<usize> {
        let mut len = 1;

        match stream.read(buf) {
            Ok(0) => {
                log::warn!("Server has shutdown or disconnected");
                return Ok(0);
            }
            Ok(n) => {
                len = n;
                log::debug!("Received message from server {n} bytes");
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                log::warn!("Read timeout reached!");
            }
            Err(e) => {
                log::error!("Error at reading stream: {e}");
            }
        }

        Ok(len)
    }

    fn process_bytes(buf: &[u8; 1024]) -> Result<(PacketType, Option<Data>)> {
        let packet: TcpPacket = bincode::deserialize(buf)
            .map_err(|e| Error::Custom(format!("Could not deserialize bytes to TcpPacket: {e}")))?;
        log::debug!("Received packet: {:?}", packet);

        assert!(packet.version == 1, "Wrong packet version used");

        let data_len = bincode::serialize(&packet.data)
            .map_err(|_| Error::Custom("Could not serialize data of packet".to_string()))?
            .len();

        match packet.packet_type {
            PacketType::Echo => {
                let packet_type = packet.packet_type;

                Ok((packet_type, None))
            }
            PacketType::Data => {
                let packet_type = packet.packet_type;
                let data = packet.data;
                assert!(
                    packet.length == data_len as u8,
                    "Packet's length does not match the data's"
                );
                Ok((packet_type, data))
            }
            PacketType::Disconnect => Ok((PacketType::Disconnect, None)),
            _ => todo!(),
        }
    }
}
