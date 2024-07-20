use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use rand::Rng;

use crate::utils::{packet_handling::*, StreamHandler};
use crate::{
    errors::{Error, Result},
    ConnectionState, PacketType,
};
use crate::{Data, TcpPacket};

type ClientID = u32;
type ClientPool = Arc<Mutex<HashMap<ClientID, ConnectionState>>>;

pub struct Server {
    ln: Option<TcpListener>,
    client_pool: ClientPool,
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

        let listener = TcpListener::bind(addr)?;

        Ok(Server {
            ln: Some(listener),
            client_pool: Arc::new(Mutex::new(HashMap::with_capacity(4))),
        })
    }

    pub fn start_server(&mut self) -> Result<()> {
        log::info!("Server has started and is reading connections");

        self.accept_loop().expect("Error at accepting connections");

        Ok(())
    }

    fn accept_loop(&mut self) -> Result<()> {
        let listener = self
            .ln
            .as_ref()
            .ok_or(Error::Custom("Listener is None".to_string()))?;

        let mut client_id_counter: ClientID = 0;

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let pool = Arc::clone(&self.client_pool);
                    let client_id = client_id_counter;
                    client_id_counter += 1;

                    log::debug!(
                        "New connection established from {}",
                        stream
                            .peer_addr()
                            .map_err(|e| Error::Custom(e.to_string()))?
                    );

                    //let (tx, rx) = mpsc::channel();
                    let mut stream = Arc::new(Mutex::new(stream));

                    {
                        let mut client_pool = pool.lock().map_err(|_| {
                            Error::Custom("Could not lock in client_pool".to_string())
                        })?;

                        client_pool.insert(client_id, ConnectionState::Listen);
                    }

                    let pool_clone = Arc::clone(&pool);

                    thread::spawn(move || {
                        if let Err(e) = Self::handle_client(client_id, &mut stream, pool_clone) {
                            log::error!("Error handling client {}: {:?}", client_id, e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error at accepting connection: {e}");
                }
            }
        }
        Ok(())
    }

    fn handle_client(
        client_id: ClientID,
        stream: &mut Arc<Mutex<TcpStream>>,
        client_pool: ClientPool,
    ) -> Result<()> {
        let mut last_activity = Instant::now();
        let timeout = Duration::from_secs(10);
        let mut response_tries = 0;

        let mut buf = [0; 1024];

        //Default Status after succesful connection
        let mut status: ConnectionState = ConnectionState::Listen;
        let mut server_seq_num = rand::thread_rng().gen_range(0..500);
        let mut server_ack_num = 0;
        loop {
            {
                let pool = client_pool.lock().map_err(|_| {
                    Error::Custom("Error at locking client in handling_client".to_string())
                })?;
                if let Some(client_status) = pool.get(&client_id) {
                    status = client_status.clone();
                }
            }
            log::info!("Status: {:?}", status);

            match status {
                ConnectionState::Listen => {
                    let buf_read = receive_packet(stream)?;
                    let packet = deserialize_packet_bytes(&buf_read)?;

                    server_ack_num = packet.seq_num + 1;

                    log::debug!("Packet rcv{:?}", packet);

                    let syn_ack_packet = TcpPacket {
                        version: 1,
                        packet_type: PacketType::SynAck,
                        seq_num: server_seq_num,
                        ack_num: server_ack_num,
                        data: None,
                    };
                    log::debug!("Server packet: {:?}", syn_ack_packet);

                    send_packet(stream, syn_ack_packet)?;
                    last_activity = Instant::now();

                    status = ConnectionState::SynRecv;
                }
                ConnectionState::SynRecv => {
                    let buf_read = receive_packet(stream)?;
                    let packet = deserialize_packet_bytes(&buf_read)?;

                    if packet.packet_type != PacketType::Ack {
                        log::warn!(
                            "Packet received from Client is not Ack: {:?} breaking handshake",
                            packet.packet_type
                        );
                        return Ok(());
                    }

                    assert!(
                        packet.ack_num == server_seq_num + 1,
                        "Received ACK does not match sent SEQ"
                    );

                    log::debug!("{:?}", packet);
                    status = ConnectionState::Established;
                    last_activity = Instant::now();
                }
                ConnectionState::Established => {
                    let client_addr = stream.peer_addr()?;
                    log::debug!("Succesfully connected to client {client_id} addr: {client_addr}");
                    log::debug!("But is time to say goodbye");
                    return Ok(());
                }

                _ => {}
            }

            // Update client status in the pool
            {
                let mut pool = client_pool
                    .lock()
                    .map_err(|_| Error::Custom("Error at locking client pool".to_string()))?;
                pool.insert(client_id, status.clone());
            }
        }
    }
}
