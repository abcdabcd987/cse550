extern crate rand;
#[macro_use] extern crate error_chain;

mod paxos;
mod locker;

pub mod errors {
    error_chain! {
        errors {
            InstanceNotExists(instance_id: usize) {
                description("instance not exists")
                display("instance not exists: '{}'", instance_id)
            }
        }
    }
}
