use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use crate::{
    errors::{Error, Result},
    PacketType, Status,
};
use crate::{Data, TcpPacket};

type ClientID = u32;
type ClientPool = Arc<Mutex<HashMap<ClientID, Status<'static>>>>;

pub struct Server<'a> {
    listen_addr: String,
    ln: Option<TcpListener>,
    client_pool: ClientPool,
    phantom: std::marker::PhantomData<&'a str>,
}

//
//struct ClientInfo{
//    client_stream: TcpStream,
//    channel
//}
//

impl<'a> Server<'a> {
    pub fn new(addr: String) -> Result<Self> {
        if !addr.contains(":") {
            return Err(Error::InvalidAddress(addr));
        }

        log::info!("Server has been initialized at: {addr}");

        Ok(Server {
            listen_addr: addr,
            ln: None,
            client_pool: Arc::new(Mutex::new(HashMap::with_capacity(4))),
            phantom: std::marker::PhantomData,
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
                    let stream = Arc::new(Mutex::new(stream));

                    {
                        let mut client_pool = pool.lock().map_err(|_| {
                            Error::Custom("Could not lock in client_pool".to_string())
                        })?;

                        client_pool.insert(client_id, Status::SendData(PacketType::Echo, None));
                    }

                    let pool_clone = Arc::clone(&pool);

                    thread::spawn(move || {
                        if let Err(e) = Self::handle_client(client_id, stream, pool_clone) {
                            log::error!("Error handling client {}: {:?}", client_id, e);
                        }
                    });

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
        stream: Arc<Mutex<TcpStream>>,
        client_pool: ClientPool,
    ) -> Result<()> {
        let mut last_activity = Instant::now();
        let timeout = Duration::from_secs(10);
        let mut response_tries = 0;

        let mut buf = [0; 1024];

        //Default Status after succesful connection
        let mut status: Status = Status::AwaitingResponse(PacketType::Echo);
        loop {
            {
                let pool = client_pool.lock().map_err(|_| {
                    Error::Custom("Error at locking client in handling_client".to_string())
                })?;
                if let Some(client_status) = pool.get(&client_id) {
                    status = client_status.clone();
                }
            }

            match status {
                Status::AwaitingResponse(PacketType::Disconnect) => {
                    Self::disconnect_client(client_id, &stream, &client_pool)?;
                }
                Status::AwaitingResponse(..) if last_activity.elapsed() > timeout => {
                    Self::send_echo(client_id, &stream, &client_pool)?;
                    last_activity = Instant::now();
                    response_tries += 1;
                }
                Status::AwaitingResponse(..) if response_tries > 3 => {
                    if let Err(e) = Self::disconnect_client(client_id, &stream, &client_pool) {
                        log::error!(
                            "Error at disconnecting client {} from client_pool: {:?}",
                            client_id,
                            e
                        );
                    }
                    log::debug!("Client{client_id} disconnected due to inactivity");
                    return Ok(());
                }
                Status::AwaitingResponse(packet_type) => {
                    let bytes_read =
                        Self::read_from_client(client_id, &stream, &mut buf, &client_pool)?;

                    if bytes_read > 0 {
                        log::debug!("Got message from client!");
                        last_activity = Instant::now();
                    }
                }
                Status::SendData(packet_type, data) => {
                    Self::send_echo(client_id, &stream, &client_pool)?;
                    log::debug!("Sent Echo package to client {client_id}");
                    last_activity = Instant::now();
                }
                Status::SendData(PacketType::Data, Some(data)) => {}
                _ => {
                    todo!()
                }
            }
        }
    }

    fn disconnect_client(
        client_id: ClientID,
        stream: &Arc<Mutex<TcpStream>>,
        client_pool: &ClientPool,
    ) -> Result<()> {
        let disconnect_packet = TcpPacket {
            version: 1,
            packet_type: PacketType::Disconnect,
            length: 0,
            data: None,
        };

        let bytes = bincode::serialize(&disconnect_packet)
            .map_err(|_| Error::Custom("Could not get bytes for disconnect packet".to_string()))?;

        let mut stream = stream
            .lock()
            .map_err(|_| Error::Custom("Error at locking stream".to_string()))?;

        if let Err(e) = stream.write_all(&bytes) {
            log::error!(
                "Could not send disconnect packet to client{}: {:?}",
                client_id,
                e
            );
        }

        let mut pool = client_pool
            .lock()
            .map_err(|_| Error::Custom("Error at locking client_pool".to_string()))?;

        pool.remove(&client_id);

        log::debug!("Client {client_id} has been removed from pool");

        Ok(())
    }

    fn read_from_client(
        client_id: ClientID,
        stream: &Arc<Mutex<TcpStream>>,
        buf: &mut [u8; 1024],
        client_pool: &ClientPool,
    ) -> Result<usize> {
        let mut stream = stream
            .lock()
            .map_err(|e| Error::Custom(format!("Error at locking stream {:?}", e)))?;

        match stream.read(buf) {
            Ok(0) => Ok(0),
            Ok(n) => Ok(n),
            Err(e) => Err(Error::Custom(format!(
                "Error at reading from client{}: {:?}",
                client_id, e
            ))),
        }
    }
    fn send_packet(
        packet_type: PacketType,
        data: Option<Data>,
        client_id: ClientID,
        stream: &Arc<Mutex<TcpStream>>,
        client_pool: &ClientPool,
    ) -> Result<()> {
        let mut tcp_packet = TcpPacket::default();

        match packet_type {
            PacketType::Echo => {
                tcp_packet = TcpPacket::echo_packet(1);
            }
            PacketType::Data => {
                tcp_packet = TcpPacket::with_data(1, data)?;
            }
            PacketType::Disconnect => {}
            _ => {}
        }

        let packet_bytes = bincode::serialize(&tcp_packet)
            .map_err(|_| Error::Custom("Error at seraliazing echo packet".to_string()))?;

        let mut stream = stream
            .lock()
            .map_err(|e| Error::Custom(format!("Error at locking stream {:?}", e)))?;

        if let Err(e) = stream.write_all(&packet_bytes) {
            log::error!("Could not send echo packet to client {client_id}: {:?}", e);
        }

        log::debug!("Echo msg sent to client:{client_id}");

        let mut client_pool = client_pool
            .lock()
            .map_err(|_| Error::Custom("Could not lock client".to_string()))?;

        if let Some(client_status) = client_pool.get_mut(&client_id) {
            *client_status = Status::AwaitingResponse(PacketType::Echo);
        }

        Ok(())
    }

    fn send_echo(
        client_id: ClientID,
        stream: &Arc<Mutex<TcpStream>>,
        client_pool: &ClientPool,
    ) -> Result<()> {
        let echo_packet = TcpPacket {
            version: 1,
            packet_type: PacketType::Echo,
            length: 0,
            data: None,
        };

        let packet_bytes = bincode::serialize(&echo_packet)
            .map_err(|_| Error::Custom("Error at seraliazing echo packet".to_string()))?;

        let mut stream = stream
            .lock()
            .map_err(|e| Error::Custom(format!("Error at locking stream {:?}", e)))?;

        if let Err(e) = stream.write_all(&packet_bytes) {
            log::error!("Could not send echo packet to client {client_id}: {:?}", e);
        }

        log::debug!("Echo msg sent to client:{client_id}");

        let mut client_pool = client_pool
            .lock()
            .map_err(|_| Error::Custom("Could not lock client".to_string()))?;

        if let Some(client_status) = client_pool.get_mut(&client_id) {
            *client_status = Status::AwaitingResponse(PacketType::Echo);
        }

        Ok(())
    }
}
