use paxos;
use locker;
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum MessagePayload<T> {
    PaxosMessage(paxos::PaxosMessage<T>),
    LockerMessage(locker::Operation),
    PrintLog,
    PrintLocks,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum MessageTarget {
    Broadcast,
    Node(paxos::NodeID),
}

#[derive(Clone, Debug)]
pub struct MessageInfo<T> {
    pub payload: MessagePayload<T>,
    pub target: MessageTarget,
    pub timeout: Option<Duration>,
}
