use rand;

pub type NodeID = String;  // TODO: maybe consider &str?
pub type InstanceID = usize;

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ProposalID(u64, u64, NodeID);

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PrepareMessage {
    pub proposer_id: NodeID,
    pub proposal_id: ProposalID,
}

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PromiseMessage<T> {
    pub acceptor_id: NodeID,
    pub proposal_id: ProposalID,
    pub last_accepted_proposal_id: ProposalID,
    pub last_accepted_value: Option<T>,
}

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ProposeMessage<T> {
    pub proposer_id: NodeID,
    pub proposal_id: ProposalID,
    pub value: T,
}

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct AcceptedMessage {
    pub acceptor_id: NodeID,
    pub proposal_id: ProposalID,
}

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct LearnMessage {
    pub learner_id: NodeID,
    pub proposal_id: ProposalID,
}

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ValueMessage<T> {
    pub learner_id: NodeID,
    pub value: T,
}

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PaxosInstanceMessage<T> {
    Prepare(PrepareMessage),
    Promise(PromiseMessage<T>),
    Propose(ProposeMessage<T>),
    Accepted(AcceptedMessage),
    Learn(LearnMessage),
    Value(ValueMessage<T>),
}

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PaxosMessage<T> {
    pub instance_id: InstanceID,
    pub message: PaxosInstanceMessage<T>,
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
