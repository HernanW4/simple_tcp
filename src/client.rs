use std::io::{self, Read};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::{io::Write, net::TcpStream};

use std::thread;
use std::time::{Duration, Instant};

use rand::Rng;

use crate::errors::{Error, Result};
use crate::utils::{packet_handling::*, StreamHandler};
use crate::{ConnectionState, Data, PacketType, TcpPacket, PACKET_VERSION};

const READ_TIMEOUT: Duration = Duration::from_secs(3);

pub struct Client {
    //Change
    pub destination_addr: String,
    stream: TcpStream,
    status: ConnectionState,
}

impl Client {
    pub fn new(addr: String) -> Result<Self> {
        if let Ok(stream) = TcpStream::connect(&addr) {
            stream
                .set_read_timeout(Some(READ_TIMEOUT))
                .map_err(|e| Error::IO(e))?;
            Ok(Client {
                destination_addr: addr,
                stream,
                status: ConnectionState::Closed,
            })
        } else {
            Err(Error::Custom(
                "Could not create a client with server address {addr}".to_string(),
            ))
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let mut last_activity = Instant::now();

        let timeout = Duration::from_secs(7);
        let mut tries = 0;

        let mut client_seq: u32 = rand::thread_rng().gen_range(0..500);
        let mut client_ack: u32 = 0;

        //Main client loop
        loop {
            log::info!("Status: {:?}", self.status);

            match self.status {
                ConnectionState::Closed => {
                    let packet = TcpPacket {
                        version: 1,
                        packet_type: PacketType::Syn,
                        seq_num: client_seq,
                        ack_num: 0,
                        data: None,
                    };

                    send_packet(&mut self.stream, packet)?;
                    self.status = ConnectionState::SynSent;
                }
                ConnectionState::SynSent => {
                    let bytes_read = receive_packet(&mut self.stream)?;
                    let packet_recv: TcpPacket = deserialize_packet_bytes(&bytes_read)?;

                    assert!(
                        packet_recv.version == PACKET_VERSION,
                        "Packet version not supported"
                    );

                    assert!(
                        packet_recv.packet_type == PacketType::SynAck,
                        "Wrong packet type"
                    );

                    if packet_recv.ack_num != client_seq + 1 {
                        log::error!("Server sent wrong ACK number. Disconnecting...");
                        return Ok(());
                    }

                    log::debug!("Packet Rcv: {:?}", packet_recv);

                    let packet = TcpPacket {
                        version: PACKET_VERSION,
                        packet_type: PacketType::Ack,
                        seq_num: client_seq + 1,
                        ack_num: packet_recv.seq_num + 1,
                        data: None,
                    };

                    log::debug!("Packet Sent: {:?}", packet);
                    send_packet(&mut self.stream, packet)?;
                    self.status = ConnectionState::Established;
                }
                ConnectionState::Established => {
                    let server_addr = self.stream.peer_addr()?;
                    log::debug!("You are fully connected to server {server_addr}");
                    log::warn!("Disconnecting now due to poor development");
                    return Ok(());
                }

                _ => {}
            }

            self.stream.flush().expect("Could not flush stream");
        }
    }
}
