use paxos;
use locker;
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize)]
pub enum MessagePayload<T> {
    PaxosMessage(paxos::PaxosMessage<T>),
    LockerMessage(locker::Operation),
    DebugPrintLog,
    DebugPrintLocks,
}

#[derive(Clone, Eq, PartialEq)]
pub enum MessageTarget {
    Broadcast,
    Node(paxos::NodeID),
}

#[derive(Clone)]
pub struct MessageInfo<T> {
    pub payload: MessagePayload<T>,
    pub target: MessageTarget,
    pub timeout: Option<Duration>,
}
