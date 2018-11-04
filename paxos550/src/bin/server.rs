#[macro_use] extern crate clap;
extern crate tokio;

use std::collections::HashMap;
use std::net::SocketAddr;
use clap::{Arg, App, SubCommand};
use tokio::prelude::*;
use tokio::net::UdpSocket;

fn main() {
    let matches = App::new("Paxos550 Server")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Starts a server that runs paxos and serves clients' locker requests.")
        .arg(Arg::with_name("id")
            .help("Paxos NodeID")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("listen")
            .help("Listening address. e.g. 0.0.0.0:9000")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("peer")
            .help("Paxos peer nodes in `id=addr` format. e.g. node1=127.0.0.1:9001")
            .required(true)
            .takes_value(true)
            .multiple(true))
        .get_matches();

    let node_id = matches.value_of("id").expect("id");
    let listen: SocketAddr = matches.value_of("listen").expect("listen").parse().expect("listen");
    let mut peers = HashMap::new();
    for peer in matches.values_of("peer").expect("peer") {
        let split: Vec<&str> = peer.split('=').collect();
        assert_eq!(split.len(), 2);
        peers.insert(split[0], split[1].parse::<SocketAddr>().expect("peer"));
    }
}