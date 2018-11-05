#[macro_use] extern crate clap;
extern crate serde_yaml;
extern crate rand;
extern crate paxos550;

use paxos550::message::*;
use paxos550::locker::Operation;

use clap::{Arg, App};
use rand::Rng;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::io;
use std::io::Write;

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
    let mut servers = HashMap::new();
    for server in matches.values_of("server").unwrap() {
        let split: Vec<&str> = server.split('=').collect();
        assert_eq!(split.len(), 2);
        servers.insert(String::from(split[0]), split[1].parse::<SocketAddr>().unwrap());
    }
    let servers_vec: Vec<_> = servers.iter().collect();

//    let local_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let mut command = String::new();
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        command.clear();
        if let Err(e) = io::stdin().read_line(&mut command) {
            println!("error: {}", e);
            break;
        }
        if command.is_empty() {
            break;
        }
        let args: Vec<_> = command.trim_end().split_whitespace().collect();
        if args.is_empty() {
            continue;
        }
        let send = |msg| {
            let data = serde_yaml::to_vec(&msg).unwrap();
            let (_, addr) = rand::thread_rng().choose(&servers_vec).unwrap();
            if let Err(e) = socket.send_to(&data, addr) {
                println!("error: {}", e);
            }
        };
        match args[0] {
            "LOCK" => {
                if args.len() != 2 {
                    println!("usage: LOCK <key>");
                    continue;
                }
                let key = args[1];
                let msg: MessagePayload<Operation> = MessagePayload::LockerMessage(
                    Operation::Lock(key.into(), node_id.into()));
                send(msg);
            },
            "UNLOCK" => {
                if args.len() != 2 {
                    println!("usage: UNLOCK <key>");
                    continue;
                }
                let key = args[1];
                let msg: MessagePayload<Operation> = MessagePayload::LockerMessage(
                    Operation::Unlock(key.into(), node_id.into()));
                send(msg);
            },
            "LOG" => {
                if args.len() != 1 {
                    println!("usage: LOG");
                    continue;
                }
                let msg: MessagePayload<Operation> = MessagePayload::DebugPrintLog;
                send(msg);
            },
            "LOCKS" => {
                if args.len() != 1 {
                    println!("usage: LOCKS");
                    continue;
                }
                let msg: MessagePayload<Operation> = MessagePayload::DebugPrintLocks;
                send(msg);
            },
            _ => {
                println!("unknown command: {:?}", args);
            }
        }
    }
}