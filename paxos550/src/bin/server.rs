extern crate tokio;
#[macro_use] extern crate futures;
#[macro_use] extern crate clap;
extern crate serde_yaml;
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

pub struct Server {
    node_id: NodeID,
    socket: UdpSocket,
    peers: HashMap<String, SocketAddr>,

    runtime: &'static mut Runtime,
    buf: [u8; MAX_UDP_SIZE],
    messages_to_send: VecDeque<MessageInfo<locker::Operation>>,
    paxos: Vec<PaxosInstance<locker::Operation>>,
    locker: locker::Locker,
}

static mut GLOBAL_SERVER: *mut Server = ptr::null_mut();
static mut GLOBAL_RUNTIME: *mut Runtime = ptr::null_mut();

fn on_timeout(msg: PaxosMessage<locker::Operation>, timeout: Duration) -> Result<()> {
    let server = unsafe { &mut *GLOBAL_SERVER };
    let instance = &mut server.paxos[msg.instance_id];
    instance.on_timeout(msg.message, timeout)
}

pub unsafe fn set_global_server(server: &mut Server) {
    // FIXME so ugly. why tokio requires futures to be 'static to be spawned?
    GLOBAL_SERVER = server;
}

pub unsafe fn set_global_runtime(runtime: &mut Runtime) {
    // FIME so ugly.
    GLOBAL_RUNTIME = runtime;
}

impl Server {
    pub fn new(node_id: NodeID, socket: UdpSocket, mut peers: HashMap<String, SocketAddr>) -> Server {
        peers.insert(node_id.clone(), socket.local_addr().unwrap());
        Server {
            node_id,
            socket,
            peers,
            runtime: unsafe { &mut *GLOBAL_RUNTIME },
            buf: [0u8; MAX_UDP_SIZE],
            messages_to_send: VecDeque::new(),
            paxos: Vec::new(),
            locker: locker::Locker::new()
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

    fn send_messages(&mut self) -> Poll<(), Error> {
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

            // send messages to self
            if *target_name == self.node_id {
                self.receive_message(message.payload)?;
                continue;  // process the next message
            }

            // send messages
            let data = serde_yaml::to_vec(&message.payload)?;
            let addr = self.peers.get(&target_name)
                .ok_or_else(|| Error::from("cannot find the peer"))?;
            match self.socket.poll_send_to(&data, addr) {
                Ok(Async::Ready(_)) => return Ok(Async::Ready(())),  // FIXME write can be incomplete
                Ok(Async::NotReady) => {
                    retry_queue.push_back(message.clone()); // FIXME clone() ugly.
                },
                Err(e) => return Err(e.into()),
            }
        }

        // add back messages to retry
        self.messages_to_send.append(&mut retry_queue);
        Ok(Async::NotReady)
    }

    fn receive_message(&mut self, message: MessagePayload<locker::Operation>) -> Poll<(), Error> {
        match message {
            MessagePayload::PaxosMessage(ref msg) => {
                if let Some(instance) = self.paxos.get_mut(msg.instance_id) {
                    if let Some(v) = instance.receive_message(&msg.message) {
                        // update the locker when the learner learns the value for the first time
                        self.locker.append_log(v);
                    }
                    instance.collect_messages_to_send(&mut self.messages_to_send);
                } else {
                    // TODO recovery
                }
            },
            MessagePayload::LockerMessage(op) => {
                let instance_id = self.paxos.len() + 1;
                let timeout = Duration::from_secs(1);
                let mut instance = PaxosInstance::new(
                    self.node_id.clone(), instance_id, self.peers.len(), timeout);
                instance.start_proposing(op);
                self.paxos.push(instance);
            },
            MessagePayload::DebugPrintLog => {
                println!("Got MessagePayload::DebugPrintLog");
                for (index, log) in self.locker.log().iter().enumerate() {
                    println!("Entry {}: {:?}", index, log);
                }
            },
            MessagePayload::DebugPrintLocks => {
                println!("Got MessagePayload::DebugPrintLocks");
                println!("{:#?}", self.locker.locks());
            }
        }
        Ok(Async::NotReady)
    }
}

impl Future for Server {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<(), Error> {
        loop {
            // try to send.
            match self.send_messages() {
                Ok(Async::Ready(())) => return Ok(Async::Ready(())),
                Ok(Async::NotReady) => (),
                Err(e) => return Err(e)
            }

            // send is not ready. try to receive.
            let (size, _addr) = try_ready!(self.socket.poll_recv_from(&mut self.buf));  // FIXME read can be incomplete
            let message: MessagePayload<locker::Operation> = serde_yaml::from_slice(&self.buf[..size])?;
            match self.receive_message(message) {
                Ok(Async::Ready(())) => (),
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => return Err(e)
            }
        }
    }
}

fn main() {
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
            .required(true)
            .takes_value(true)
            .multiple(true))
        .get_matches();

    let node_id = matches.value_of("id").unwrap();
    let listen: SocketAddr = matches.value_of("listen").unwrap().parse().unwrap();
    let mut peers = HashMap::new();
    for peer in matches.values_of("peer").unwrap() {
        let split: Vec<&str> = peer.split('=').collect();
        assert_eq!(split.len(), 2);
        peers.insert(String::from(split[0]), split[1].parse::<SocketAddr>().unwrap());
    }
    let socket = UdpSocket::bind(&listen).unwrap();
    println!("Listening on: {}", socket.local_addr().unwrap());

    let mut runtime = tokio::runtime::Builder::new()
        .core_threads(1).build().unwrap();
    unsafe { set_global_runtime(&mut runtime); }
    let mut server = Server::new(node_id.to_string(), socket, peers);
    unsafe { set_global_server(&mut server); }
    runtime.spawn(server.map_err(|e| println!("error: {}", e)));
    runtime.shutdown_on_idle().wait().unwrap();
}