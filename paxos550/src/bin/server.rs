extern crate tokio;
#[macro_use] extern crate clap;
extern crate paxos550;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::thread;
use clap::{Arg, App};
use tokio::prelude::*;
use tokio::net::UdpSocket;
use std::time;
use paxos550::*;

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

    println!("Sleeping...");
    thread::sleep(time::Duration::from_secs(5));
    println!("Starting tokio...");
    let server = Server::new(node_id.to_string(), socket, peers);
    tokio::run(server.map_err(|e| println!("server error = {:?}", e)));
}