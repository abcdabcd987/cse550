extern crate rand;
extern crate tokio;
#[macro_use] extern crate futures;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate serde_derive;
extern crate serde_yaml;
extern crate core;

mod paxos;
mod locker;
mod network;

pub mod errors {
    use serde_yaml;
    use std;
    use tokio;

    error_chain! {
        errors {
            InstanceNotExists(instance_id: usize) {
                description("instance not exists")
                display("instance not exists: '{}'", instance_id)
            }
        }
        foreign_links {
            SerdeError(serde_yaml::Error);
            IoError(std::io::Error);
            TokioTimerError(tokio::timer::Error);
        }
    }
}

pub use network::Server;
pub use paxos::*;
