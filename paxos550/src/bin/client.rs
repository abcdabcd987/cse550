#[macro_use] extern crate clap;
extern crate serde_yaml;
extern crate rand;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate rustyline;
extern crate paxos550;

use paxos550::message::*;
use paxos550::paxos::NodeID;
use paxos550::locker::{Operation, LogEntry};

use clap::{Arg, App};
use rand::Rng;
use rustyline::Editor;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::net::UdpSocket;

fn print_usage() {
    println!(r#"USAGE:
    LOCK <key> [server]           Request to lock <key>
    UNLOCK <key> [server]         Request to unlock <key>
    LOG [server]                  Query the log applied by the state machine
    LOCKS [server]                Query what are locked
    TOTAL [server]                Query the number of paxos instances
    "#);
}

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

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

    info!("Client id: {}", node_id);
    for (name, addr) in &servers {
        info!("Server {}: {}", name, addr);
    }

    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let mut buf = [0u8; 8192];
    let mut rl = Editor::<()>::new();
    let prompt = format!("{}> ", node_id);
    print_usage();
    loop {
        let command = if let Ok(line) = rl.readline(&prompt) {
            line
        } else {
            break;
        };
        rl.add_history_entry(command.as_ref());
        let args: Vec<_> = command.trim_end().split_whitespace().collect();
        if args.is_empty() {
            continue;
        }
        let send = |msg, server: Option<&&str>| {
            let data = serde_yaml::to_vec(&msg).unwrap();
            let name = if let Some(name) = server {
                *name
            } else {
                let (name, _) = rand::thread_rng().choose(&servers_vec).unwrap();
                name.as_str()
            };
            match servers.get(name) {
                None => {
                    println!("cannot find server {}", name);
                    return false;
                },
                Some(addr) => {
                    info!("sent to {}: {:?}", addr, msg);
                    if let Err(e) = socket.send_to(&data, addr) {
                        println!("error: {}", e);
                    }
                },
            }
            return true;
        };
        match args[0] {
            "LOCK" => {
                let key = if let Some(&key) = args.get(1) {
                    key
                } else {
                    println!("usage: LOCK <key> [server]");
                    continue;
                };
                let msg: MessagePayload<Operation> = MessagePayload::LockerMessage(
                    Operation::Lock(key.into(), node_id.into()));
                send(msg, args.get(2));
            },
            "UNLOCK" => {
                let key = if let Some(&key) = args.get(1) {
                    key
                } else {
                    println!("usage: UNLOCK <key> [server]");
                    continue;
                };
                let msg: MessagePayload<Operation> = MessagePayload::LockerMessage(
                    Operation::Unlock(key.into(), node_id.into()));
                send(msg, args.get(2));
            },
            "LOG" => {
                let msg: MessagePayload<Operation> = MessagePayload::PrintLog;
                if !send(msg, args.get(1)) {
                    continue;
                }
                let mut len = 0;
                loop {
                    match socket.recv_from(&mut buf[len..]) {
                        Ok((size, addr)) => {
                            len += size;
                            let data: Result<Vec<LogEntry>, _> = serde_yaml::from_slice(&buf);
                            if let Ok(res) = data {
                                println!("Log from {}:", addr);
                                for entry in &res {
                                    println!("{:?}", entry);
                                }
                                println!("({} LogEntry in total)", res.len());
                                break;
                            }
                        },
                        Err(e) => {
                            println!("error: {}", e);
                            break;
                        },
                    }
                }
            },
            "LOCKS" => {
                let msg: MessagePayload<Operation> = MessagePayload::PrintLocks;
                if !send(msg, args.get(1)) {
                    continue;
                }
                let mut len = 0;
                loop {
                    match socket.recv_from(&mut buf[len..]) {
                        Ok((size, addr)) => {
                            len += size;
                            let data: Result<HashMap<String, NodeID>, _> = serde_yaml::from_slice(&buf[..len]);
                            if let Ok(res) = data {
                                println!("Locks from {}:", addr);
                                for (key, node) in &res {
                                    println!("{}\t=>\t{}", key, node);
                                }
                                break;
                            }
                        },
                        Err(e) => {
                            println!("error: {}", e);
                            break;
                        },
                    }
                }
            },
            "TOTAL" => {
                let msg: MessagePayload<Operation> = MessagePayload::PrintTotalInstances;
                if !send(msg, args.get(1)) {
                    continue;
                }
                let mut len = 0;
                loop {
                    match socket.recv_from(&mut buf[len..]) {
                        Ok((size, addr)) => {
                            len += size;
                            let data: Result<usize, _> = serde_yaml::from_slice(&buf[..len]);
                            if let Ok(res) = data {
                                println!("Total instances from {}: {}", addr, res);
                                break;
                            }
                        },
                        Err(e) => {
                            println!("error: {}", e);
                            break;
                        },
                    }
                }
            },
            "HELP" => {
                print_usage();
            },
            _ => {
                println!("unknown command: {:?}", args);
            }
        }
    }
}