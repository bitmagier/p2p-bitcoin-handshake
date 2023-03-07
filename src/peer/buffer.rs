use std::io;
use std::net::{IpAddr, SocketAddr};

use crate::peer::wire_protocol::NodeServiceSet;

pub(super) struct ByteBufferParser<'a> {
    buffer: &'a [u8],
    pos: usize,
}

impl<'a> ByteBufferParser<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        let pos = 0;
        ByteBufferParser { buffer, pos }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.pos
    }

    pub fn skip_bytes(&mut self, count: usize) -> io::Result<()> {
        self.eof_check(count)?;
        self.pos += count;
        Ok(())
    }

    pub fn read(&mut self, size: usize) -> io::Result<&'a [u8]> {
        self.eof_check(size)?;
        let range = self.pos..self.pos + size;
        self.pos += size;
        Ok(&self.buffer[range])
    }

    pub fn read_u32_le(&mut self) -> io::Result<u32> {
        Ok(u32::from_le_bytes(
            self.read(4)?.try_into().unwrap()
        ))
    }

    pub fn read_i32_le(&mut self) -> io::Result<i32> {
        Ok(i32::from_le_bytes(
            self.read(4)?.try_into().unwrap()
        ))
    }

    pub fn read_u64_le(&mut self) -> io::Result<u64> {
        Ok(u64::from_le_bytes(
            self.read(8)?.try_into().unwrap()
        ))
    }

    pub fn read_i64_le(&mut self) -> io::Result<i64> {
        Ok(i64::from_le_bytes(
            self.read(8)?.try_into().unwrap()
        ))
    }

    fn read_u16_be(&mut self) -> io::Result<u16> {
        Ok(u16::from_be_bytes(
            self.read(2)?.try_into().unwrap()
        ))
    }

    // without time field
    pub fn parse_net_addr(&mut self) -> io::Result<(NodeServiceSet, SocketAddr)> {
        let services_mask = self.read_u64_le()?;
        let ip: [u8; 16] = self.read(16)?.try_into().unwrap();
        let ip = IpAddr::from(ip);
        let port = self.read_u16_be()?;
        Ok((
            NodeServiceSet::from_bitmask(services_mask),
            SocketAddr::new(ip, port)
        ))
    }

    /// 1+  length  varint  (https://en.bitcoin.it/wiki/Protocol_documentation#Variable_length_integer)
    /// ?   string  char[]
    // pub fn read_var_string(&self) -> io::Result<String> {
    //     todo!()
    // }

    fn eof_check(&self, want_bytes: usize) -> io::Result<()> {
        if self.remaining() < want_bytes {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("can not read {} bytes from buffer of size {}", want_bytes, self.buffer.len()))
            )
        } else {
            Ok(())
        }
    }
}


pub(super) struct ByteBufferComposer {
    buffer: Vec<u8>,
}

impl ByteBufferComposer {
    pub fn new() -> Self {
        ByteBufferComposer { buffer: vec![] }
    }

    pub fn result(self) -> Vec<u8> {
        self.buffer
    }

    pub fn append(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    /// net address struct without time field
    pub fn append_net_addr(&mut self, service: &NodeServiceSet, addr: &SocketAddr) {
        self.append(&service.as_bitmask().to_le_bytes());
        let ipv6_octets = match &addr.ip() {
            IpAddr::V4(ip) => ip.to_ipv6_mapped().octets(),
            IpAddr::V6(ip) => ip.octets()
        };
        self.append(&ipv6_octets);
        self.append(&addr.port().to_be_bytes());
    }
}

pub(super) struct IOBuffer {
    buffer: [u8; 1024],
    /// length of valid content (starts at index 0)
    mark: usize,
}

impl IOBuffer {
    pub fn content(&self) -> &[u8] {
        &self.buffer[..self.mark]
    }

    pub fn expose_writable_part(&mut self) -> &mut [u8] {
        &mut self.buffer[self.mark..]
    }

    /// Increase buffer mark my `size'.
    /// This method is used to make the buffer aware of new bytes written into slice returned by [Self::expose_writable_part]
    pub fn register_added_content(&mut self, size: usize) {
        assert!(self.mark + size <= self.buffer.len());
        self.mark += size;
    }

    /// removes `size` bytes from beginning of buffer. reduces `mark` by `size`
    pub fn shift_left(&mut self, size: usize) {
        assert!(size <= self.mark);
        self.buffer.rotate_left(size);
        self.mark -= size;
    }
}

impl Default for IOBuffer {
    fn default() -> Self {
        IOBuffer {
            buffer: [0_u8; 1024],
            mark: 0,
        }
    }
}
