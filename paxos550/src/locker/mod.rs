use std::collections::HashMap;
use std::vec::Vec;
use std::cmp::Eq;
use std::hash::Hash;

#[derive(Clone)]
pub enum Operation<K, V> {
    Lock(K, V),
    Unlock(K)
}

#[derive(Clone)]
pub struct LogEntry<K, V> {
    op: Operation<K, V>,
    valid: bool,
}

pub struct Locker<K, V> {
    locks: HashMap<K, V>,
    log: Vec<LogEntry<K, V>>
}

impl<K: Hash + Eq + Clone, V: Clone> Locker<K, V> {
    pub fn new() -> Locker<K, V> {
        Locker {
            locks: HashMap::new(),
            log: Vec::new()
        }
    }

    pub fn append_log(&mut self, op: Operation<K, V>) {
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
                self.locks.remove(&key).is_some()
            },
        };
        self.log.push(LogEntry { op, valid });
    }

    pub fn log(&self) -> &Vec<LogEntry<K, V>> {
        &self.log
    }

    pub fn locks(&self) -> &HashMap<K, V> {
        &self.locks
    }
}
