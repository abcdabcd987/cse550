use std::{env, io};
use std::net::SocketAddr;
use tokio::prelude::*;
use tokio::net::UdpSocket;
use std::collections::HashMap;
use serde_yaml;

use paxos::*;
use errors::*;
use locker;
use super::message::MessagePayload;

const MAX_UDP_SIZE: usize = 1500 - 20 - 8;

pub struct Client {
    node_id: NodeID,
    socket: UdpSocket,
    servers: HashMap<String, SocketAddr>,
    buf: [u8; MAX_UDP_SIZE],
}

impl Client {
    pub fn new(node_id: NodeID, socket: UdpSocket, servers: HashMap<String, SocketAddr>) -> Client {
        Client {
            node_id,
            socket,
            servers,
            buf: [0u8; MAX_UDP_SIZE],
        }
    }
}

impl Future for Client {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<(), Error> {
        loop {
            let (size, addr) = try_ready!(self.socket.poll_recv_from(&mut self.buf));
            let message: MessagePayload<locker::Operation> = serde_yaml::from_slice(&self.buf[..size])?;
        }
    }
}


