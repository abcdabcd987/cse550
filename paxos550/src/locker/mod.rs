use std::collections::HashMap;
use std::vec::Vec;

use paxos::NodeID;

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum Operation {
    Lock(String, NodeID),
    Unlock(String, NodeID)
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
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

    pub fn append_log(&mut self, op: &Operation) {
        let mut valid = false;
        match op {
            Operation::Lock(ref key, ref value) => {
                if !self.locks.contains_key(key) {
                    self.locks.insert(key.clone(), value.clone());
                    valid = true;
                }
            },
            Operation::Unlock(ref key, ref node) => {
                match self.locks.get(key) {
                    Some(owner) if owner == node => valid = true,
                    _ => ()
                }
                if valid {
                    self.locks.remove(key);  // FIXME ugly.
                }
            },
        }
        self.log.push(LogEntry { op: op.clone(), valid });
    }

    pub fn log(&self) -> &Vec<LogEntry> {
        &self.log
    }

    pub fn locks(&self) -> &HashMap<String, NodeID> {
        &self.locks
    }
}
