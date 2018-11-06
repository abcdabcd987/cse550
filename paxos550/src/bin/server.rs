extern crate tokio;
#[macro_use] extern crate futures;
#[macro_use] extern crate clap;
extern crate serde_yaml;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate paxos550;

use paxos550::paxos::*;
use paxos550::locker;
use paxos550::errors::*;
use paxos550::network::message::*;

use tokio::prelude::*;
use tokio::net::UdpSocket;
use tokio::timer::Delay;
use tokio::runtime::Runtime;
use clap::{Arg, App};

use std::time::Duration;
use std::time::Instant;
use std::net::SocketAddr;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::ptr;

const MAX_UDP_SIZE: usize = 1500 - 20 - 8;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);

pub struct Server {
    node_id: NodeID,
    socket: UdpSocket,
    peers: HashMap<String, SocketAddr>,

    init: bool,
    runtime: &'static mut Runtime,
    buf: [u8; MAX_UDP_SIZE],
    messages_to_send: VecDeque<MessageInfo<locker::Operation>>,
    paxos: Vec<PaxosInstance<locker::Operation>>,
    locker: locker::Locker,
    next_log_to_apply: usize,
}

static mut GLOBAL_SERVER: *mut Server = ptr::null_mut();
static mut GLOBAL_RUNTIME: *mut Runtime = ptr::null_mut();

fn on_timeout(msg: PaxosMessage<locker::Operation>, timeout: Duration) -> Result<()> {
    let server = unsafe { &mut *GLOBAL_SERVER };
    {
        let instance = &mut server.paxos[msg.instance_id];
        instance.on_timeout(msg.message, timeout)?;
        instance.collect_messages_to_send(&mut server.messages_to_send);
    }
    loop {
        match server.send_messages() {
            Ok(Async::Ready(())) => (),
            Ok(Async::NotReady) => return Ok(()),
            Err(e) => return Err(e),
        }
    }
}

pub unsafe fn set_global_server(server: &mut Server) {
    // FIXME so ugly. why tokio requires futures to be 'static to be spawned?
    GLOBAL_SERVER = server;
}

pub unsafe fn set_global_runtime(runtime: &mut Runtime) {
    // FIXME so ugly.
    GLOBAL_RUNTIME = runtime;
}

impl Server {
    pub fn new(node_id: NodeID, socket: UdpSocket, mut peers: HashMap<String, SocketAddr>) -> Server {
        peers.insert(node_id.clone(), socket.local_addr().unwrap());
        let empty_instance = PaxosInstance::new(
            node_id.clone(), 0, peers.len(), Duration::default());
        Server {
            node_id,
            socket,
            peers,
            init: true,
            runtime: unsafe { &mut *GLOBAL_RUNTIME },
            buf: [0u8; MAX_UDP_SIZE],
            messages_to_send: VecDeque::new(),
            paxos: vec![empty_instance],
            locker: locker::Locker::new(),
            next_log_to_apply: 1
        }
    }

    fn setup_timeout_trigger(&mut self, now: Instant, message: MessageInfo<locker::Operation>) {
        // setup timeout trigger
        if let Some(timeout) = message.timeout {
            if let MessagePayload::PaxosMessage(msg) = message.payload {
                let deadline = now + timeout;
                let timer = Delay::new(deadline)
                    .map_err(|e| e.into())
                    .and_then(move |_| {
                        on_timeout(msg, timeout)
                    })
                    .map_err(|e: Error| println!("timer error {}", e));
                self.runtime.spawn(timer);
            }
        }
    }

    pub fn send_messages(&mut self) -> Poll<(), Error> {
        let mut not_ready = true;
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
                            timeout: None  // clear timeout here
                        })
                    }

                    // setup timeout trigger
                    self.setup_timeout_trigger(now, message);

                    // process the peer-to-peer messages
                    continue;
                },
                MessageTarget::Node(ref x) => x.clone(),
            };

            // setup timeout trigger
            self.setup_timeout_trigger(now, message.clone());

            debug!("send message to {}: {:?}", target_name, message.payload);

            // send messages to self
            if *target_name == self.node_id {
                self.receive_message(message.payload, "127.0.0.1:1234".parse().unwrap())?;  // FIXME don't need addr here
                continue;  // process the next message
            }

            // send messages
            let data = serde_yaml::to_vec(&message.payload)?;
            let addr = self.peers.get(&target_name)
                .ok_or_else(|| Error::from("cannot find the peer"))?;
            match self.socket.poll_send_to(&data, addr) {
                Ok(Async::Ready(_)) => not_ready = false,  // FIXME write can be incomplete
                Ok(Async::NotReady) => {
                    retry_queue.push_back(message.clone()); // FIXME clone() ugly.
                },
                Err(e) => return Err(e.into()),
            }
        }

        // add back messages to retry
        self.messages_to_send.append(&mut retry_queue);
        Ok(if not_ready { Async::NotReady } else { Async::Ready(()) })
    }

    fn receive_message(&mut self, message: MessagePayload<locker::Operation>, addr: SocketAddr) -> Poll<(), Error> {
        debug!("got message from {}: {:?}", addr, message);
        match message {
            MessagePayload::PaxosMessage(ref msg) => {
                // create all the missing instances
                let next_instance_id = self.paxos.len();
                for instance_id in next_instance_id ..= msg.instance_id {
                    let mut instance = PaxosInstance::new(
                        self.node_id.clone(), instance_id, self.peers.len(), DEFAULT_TIMEOUT);
                    instance.learn_final_consensus();
                    instance.collect_messages_to_send(&mut self.messages_to_send);
                    self.paxos.push(instance);
                }

                // handle the message
                let apply_log;
                {
                    let instance = &mut self.paxos[msg.instance_id];
                    apply_log = instance.receive_message(&msg.message).is_some();
                    instance.collect_messages_to_send(&mut self.messages_to_send);
                }

                // update the locker when the learner learns the value for the first time
                if apply_log {
                    let total_instances = self.paxos.len() - 1;
                    while self.next_log_to_apply <= total_instances {
                        let inst = &mut self.paxos[self.next_log_to_apply];
                        if let Some(v) = inst.value() {
                            self.locker.append_log(&v);
                        }
                        self.next_log_to_apply += 1;
                    }
                }
            },
            MessagePayload::LockerMessage(op) => {
                let instance_id = self.paxos.len();
                let mut instance = PaxosInstance::new(
                    self.node_id.clone(), instance_id, self.peers.len(), DEFAULT_TIMEOUT);
                instance.start_proposing(op);
                instance.collect_messages_to_send(&mut self.messages_to_send);
                self.paxos.push(instance);
            },
            MessagePayload::PrintLog => {
                // FIXME unify send message
                let data = serde_yaml::to_vec(self.locker.log())?;
                match self.socket.poll_send_to(&data, &addr) {
                    Ok(Async::Ready(_)) => return Ok(Async::Ready(())),  // FIXME write can be incomplete
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(e) => return Err(e.into()),
                }
            }
            MessagePayload::PrintLocks => {
                let data = serde_yaml::to_vec(self.locker.locks())?;
                match self.socket.poll_send_to(&data, &addr) {
                    Ok(Async::Ready(_)) => return Ok(Async::Ready(())),  // FIXME write can be incomplete
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(e) => return Err(e.into()),
                }
            }
        }
        Ok(Async::Ready(()))
    }
}

impl Future for Server {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<(), Error> {
        if self.init {
            unsafe { set_global_server(self); }  // FIXME ugly
            self.init = false;
        }
        debug!("poll");
        loop {
            debug!("poll > send");
            let mut not_ready = true;

            // try to send.
            match self.send_messages() {
                Ok(Async::Ready(())) => not_ready = false,
                Ok(Async::NotReady) => (),
                Err(e) => return Err(e)
            }

            debug!("poll > receive");
            // send is not ready. try to receive.
            let (size, addr) = try_ready!(self.socket.poll_recv_from(&mut self.buf));  // FIXME read can be incomplete
            let message: MessagePayload<locker::Operation> = serde_yaml::from_slice(&self.buf[..size])?;
            match self.receive_message(message, addr) {
                Ok(Async::Ready(())) => not_ready = false,
                Ok(Async::NotReady) => (),
                Err(e) => return Err(e)
            }

            if not_ready {
                return Ok(Async::NotReady);
            }
        }
    }
}

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let matches = App::new("Paxos550 Lock Service Server and Paxos Server")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Starts a server that runs paxos and serves clients' locker requests.")
        .arg(Arg::with_name("id")
            .long("id")
            .help("Paxos NodeID")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("listen")
            .long("listen")
            .help("Listening address. e.g. 0.0.0.0:9000")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("peer")
            .long("peer")
            .help("Paxos peer nodes in `id=addr` format. e.g. node1=127.0.0.1:9001")
            .required(false)
            .takes_value(true)
            .multiple(true))
        .get_matches();

    let node_id = matches.value_of("id").unwrap();
    let listen: SocketAddr = matches.value_of("listen").unwrap().parse().unwrap();
    let mut peers = HashMap::new();
    if let Some(peers_str) = matches.values_of("peer") {
        for peer in peers_str {
            let split: Vec<&str> = peer.split('=').collect();
            assert_eq!(split.len(), 2);
            peers.insert(String::from(split[0]), split[1].parse::<SocketAddr>().unwrap());
        }
    }
    let socket = UdpSocket::bind(&listen).unwrap();
    info!("Server {} listening on: {}", node_id, socket.local_addr().unwrap());
    for (name, addr) in &peers {
        info!("Peer {}: {}", name, addr);
    }

    let mut runtime = tokio::runtime::Builder::new()
        .core_threads(1).build().unwrap();
    unsafe { set_global_runtime(&mut runtime); }
    let server = Server::new(node_id.to_string(), socket, peers);
    runtime.spawn(server.map_err(|e| error!("error: {}", e)));
    runtime.shutdown_on_idle().wait().unwrap();
}