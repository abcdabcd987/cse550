extern crate tokio;
#[macro_use] extern crate clap;
extern crate paxos550;

use std::collections::HashMap;
use std::net::SocketAddr;
use clap::{Arg, App};
use tokio::prelude::*;
use tokio::net::UdpSocket;

fn main() {
    let matches = App::new("Paxos550 Lock Service Client")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Starts a interactive lock service client.")
        .arg(Arg::with_name("id")
            .long("id")
            .help("Unique client name")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("server")
            .long("server")
            .help("Server nodes in `id=addr` format. e.g. node1=127.0.0.1:9001")
            .required(true)
            .takes_value(true)
            .multiple(true))
        .get_matches();

    let node_id = matches.value_of("id").unwrap();
    let mut peers = HashMap::new();
    for peer in matches.values_of("server").unwrap() {
        let split: Vec<&str> = peer.split('=').collect();
        assert_eq!(split.len(), 2);
        peers.insert(String::from(split[0]), split[1].parse::<SocketAddr>().unwrap());
    }

    let local_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let socket = UdpSocket::bind(&local_addr).unwrap();
}