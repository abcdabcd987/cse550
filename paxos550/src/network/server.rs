use paxos::*;
use errors::*;
use locker;
use super::message::*;

use serde_yaml;
use tokio;
use tokio::prelude::*;
use tokio::net::UdpSocket;
use tokio::timer::Delay;

use std::{env, io};
use std::time::Instant;
use std::net::SocketAddr;
use std::collections::HashMap;
use std::collections::VecDeque;

const MAX_UDP_SIZE: usize = 1500 - 20 - 8;

pub struct Server {
    node_id: NodeID,
    socket: UdpSocket,
    peers: HashMap<String, SocketAddr>,
    buf: [u8; MAX_UDP_SIZE],
    messages_to_send: VecDeque<MessageInfo<locker::Operation>>,
    paxos: Vec<PaxosInstance<locker::Operation>>,
}

impl Server {
    pub fn new(node_id: NodeID, socket: UdpSocket, peers: HashMap<String, SocketAddr>) -> Server {
        Server {
            node_id,
            socket,
            peers,
            buf: [0u8; MAX_UDP_SIZE],
            messages_to_send: VecDeque::new(),
            paxos: Vec::new()
        }
    }
}

impl Future for Server {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<(), Error> {
        loop {
            // send
            let mut retry_queue = VecDeque::new();
            let now = Instant::now();
            while let Some(message) = self.messages_to_send.pop_front() {
                let target_name = match message.target {
                    MessageTarget::Broadcast => {
                        // break broadcast messages into peer-to-peer messages
                        for (name, _) in &self.peers {
                            self.messages_to_send.push_front(MessageInfo {
                                payload: message.payload.clone(),
                                target: MessageTarget::Node(name.clone()),
                                timeout: None
                            })
                        }

                        // setup timeout trigger
                        if let Some(timeout) = message.timeout {
                            if let MessagePayload::PaxosMessage(msg) = message.payload {
                                let deadline = now + timeout;
                                let timer = Delay::new(deadline)
                                    .map_err(|e| e.into())
                                    .and_then(|_| {
                                        let instance = &mut self.paxos[msg.instance_id];
                                        instance.on_timeout(msg.message, timeout)
                                    });
                            }
                        }

                        // process the peer-to-peer messages
                        continue;
                    },
                    MessageTarget::Node(ref x) => x,
                };

                // send messages
                let data = serde_yaml::to_vec(&message.payload)?;
                let addr = self.peers.get(target_name)
                    .ok_or_else(|| Error::from("cannot find the peer"))?;
                match self.socket.poll_send_to(&data, addr) {
                    Ok(Async::Ready(_)) => return Ok(Async::Ready(())),  // FIXME write can be incomplete
                    Ok(Async::NotReady) => {
                        retry_queue.push_back(message.clone()); // FIXME clone() ugly.
                    },
                    Err(e) => return Err(e.into()),
                }
            }

            // recv
            let (size, addr) = try_ready!(self.socket.poll_recv_from(&mut self.buf));  // FIXME read can be incomplete
            let message: MessagePayload<locker::Operation> = serde_yaml::from_slice(&self.buf[..size])?;
            match message {
                MessagePayload::PaxosMessage(ref msg) => {
                    if let Some(instance) = self.paxos.get_mut(msg.instance_id) {
                        instance.receive_message(&msg.message);
                        instance.collect_messages_to_send(&mut self.messages_to_send);
                    } else {
                        // TODO recovery
                    }
                },
                _ => {
                    // TODO locker service
                },
            }
        }
    }
}


