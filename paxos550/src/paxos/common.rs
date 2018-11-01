use rand;

pub type NodeID = String;  // TODO: maybe consider &str?
pub type InstanceID = usize;

#[derive(Clone, PartialOrd, PartialEq)]
pub struct ProposalID(u64, u64, NodeID);

pub struct PrepareMessage {
    pub instance_id: InstanceID,
    pub proposer_id: NodeID,
    pub proposal_id: ProposalID,
}

pub struct PromiseMessage<T> {
    pub instance_id: InstanceID,
    pub acceptor_id: NodeID,
    pub proposal_id: ProposalID,
    pub last_accepted_proposal_id: ProposalID,
    pub last_accepted_value: Option<T>,
}

pub struct ProposeMessage<T> {
    pub instance_id: InstanceID,
    pub proposer_id: NodeID,
    pub proposal_id: ProposalID,
    pub value: T,
}

pub struct AcceptedMessage {
    pub instance_id: InstanceID,
    pub acceptor_id: NodeID,
    pub proposal_id: ProposalID,
}

pub struct LearnValueMessage {
    pub instance_id: InstanceID,
    pub learner_id: NodeID,
    pub proposal_id: ProposalID,
}

pub struct ValueMessage<T> {
    pub instance_id: InstanceID,
    pub value: T,
}

pub enum Message<T> {
    None,
    Prepare(PrepareMessage),
    Promise(PromiseMessage<T>),
    Propose(ProposeMessage<T>),
    Accepted(AcceptedMessage),
    LearnValue(LearnValueMessage),
    Value(ValueMessage<T>),
}

impl ProposalID {
    pub fn new(round: u64, proposer_id: NodeID) -> ProposalID {
        ProposalID(round, rand::random(), proposer_id)
    }

    pub fn round(&self) -> u64 {
        self.0
    }

    pub fn proposer_id(&self) -> NodeID {
        self.2.clone()
    }
}
