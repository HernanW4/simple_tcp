use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::{Arc, Mutex},
};

use crate::{
    errors::{Error, Result},
    TcpPacket,
};

use std::net::SocketAddr;

pub trait StreamHandler {
    fn read_stream(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn write_stream(&mut self, buf: &[u8]) -> Result<usize>;
    fn peer_addr(&self) -> Result<SocketAddr>;
}

impl StreamHandler for TcpStream {
    fn read_stream(&mut self, buf: &mut [u8]) -> Result<usize> {
        Ok(self.read(buf)?)
    }
    fn write_stream(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(self.write(buf)?)
    }
    fn peer_addr(&self) -> Result<SocketAddr> {
        Ok(self.peer_addr()?)
    }
}

impl StreamHandler for Arc<Mutex<TcpStream>> {
    fn read_stream(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut stream = self.lock().map_err(|e| {
            Error::Custom(format!(
                "Could not get a lock on stream for reading {:?}",
                e
            ))
        })?;

        Ok(stream.read(buf)?)
    }

    fn write_stream(&mut self, buf: &[u8]) -> Result<usize> {
        let mut stream = self.lock().map_err(|e| {
            Error::Custom(format!(
                "Could not get a lock on stream for writing {:?}",
                e
            ))
        })?;

        Ok(stream.write(buf)?)
    }

    fn peer_addr(&self) -> Result<SocketAddr> {
        let stream = self.lock().map_err(|e| {
            Error::Custom(format!(
                "Could not get a lock on stream for writing {:?}",
                e
            ))
        })?;

        Ok(stream.peer_addr()?)
    }
}

pub mod packet_handling {
    use super::*;
    pub fn receive_packet<T: StreamHandler>(stream: &mut T) -> Result<Vec<u8>> {
        let mut buf = vec![0; 1024];

        let bytes_read = stream.read_stream(&mut buf)?;

        let source = stream.peer_addr().expect("Could not get address of packet");

        log::debug!("Read {bytes_read} bytes from addr: {source}");

        buf.truncate(bytes_read);

        Ok(buf)
    }

    pub fn deserialize_packet_bytes(buf: &[u8]) -> Result<TcpPacket> {
        bincode::deserialize(buf)
            .map_err(|e| Error::Custom(format!("Could not desrialize bytes to packet: {}", e)))
    }
    pub fn send_packet<T: StreamHandler>(stream: &mut T, packet: TcpPacket) -> Result<()> {
        let packet_buf = bincode::serialize(&packet)
            .map_err(|_| Error::Custom("Could not serialize packet into bytes".to_string()))?;

        let bytes_written = stream.write_stream(&packet_buf)?;

        let destination = stream
            .peer_addr()
            .expect("Could not get address destination");

        log::debug!("Written {bytes_written} bytes to addr: {destination}");

        Ok(())
    }
}
