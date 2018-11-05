use std::collections::HashMap;
use std::vec::Vec;
use std::cmp::Eq;
use std::hash::Hash;

use paxos::NodeID;

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Operation {
    Lock(String, NodeID),
    Unlock(String)
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogEntry {
    op: Operation,
    valid: bool,
}

pub struct Locker {
    locks: HashMap<String, NodeID>,
    log: Vec<LogEntry>
}

impl Locker {
    pub fn new() -> Locker {
        Locker {
            locks: HashMap::new(),
            log: Vec::new()
        }
    }

    pub fn append_log(&mut self, op: Operation) {
        let valid = match op {
            Operation::Lock(ref key, ref value) => {
                if !self.locks.contains_key(key) {
                    self.locks.insert(key.clone(), value.clone());
                    true
                } else {
                    false
                }
            },
            Operation::Unlock(ref key) => {
                self.locks.remove(key).is_some()
            },
        };
        self.log.push(LogEntry { op, valid });
    }

    pub fn log(&self) -> &Vec<LogEntry> {
        &self.log
    }

    pub fn locks(&self) -> &HashMap<String, NodeID> {
        &self.locks
    }
}
