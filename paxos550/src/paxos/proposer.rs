use super::common::*;
use errors::*;
use std::collections::HashSet;

pub struct Proposer<T> {
    proposer_id: NodeID,
    majority_size: usize,
    proposal_id: ProposalID,
    highest_proposal_id: ProposalID,
    received_promises: HashSet<NodeID>,
    value: Option<T>,
}

impl<T: Clone> Proposer<T> {
    pub fn new(proposer_id: NodeID, cluster_size: usize) -> Proposer<T> {
        let highest_proposal_id = ProposalID::new(0, proposer_id.clone());
        Proposer {
            proposer_id,
            majority_size: (cluster_size + 1) / 2,
            proposal_id: highest_proposal_id.clone(),
            highest_proposal_id,
            received_promises: HashSet::new(),
            value: None
        }
    }

    pub fn observe_proposal(&mut self, proposal_id: &ProposalID) -> Result<()> {
        if *proposal_id > self.highest_proposal_id {
            self.highest_proposal_id = proposal_id.clone();
        }
        Ok(())
    }

    pub fn prepare(&mut self) -> Result<PrepareMessage> {
        self.proposal_id = ProposalID::new(self.highest_proposal_id.round() + 1,
                                           self.proposer_id.clone());
        self.highest_proposal_id = self.proposal_id.clone();
        self.received_promises.clear();
        Ok(PrepareMessage {
            proposer_id: self.proposer_id.clone(),
            proposal_id: self.proposal_id.clone()
        })
    }

    pub fn receive_promise(&mut self, promise: &PromiseMessage<T>) -> Result<Message<T>> {
        self.observe_proposal(&promise.proposal_id)?;
        if self.proposal_id == promise.proposal_id && !self.received_promises.contains(&promise.acceptor_id) {
            self.received_promises.insert(promise.acceptor_id.clone());
            if promise.last_accepted_proposal_id > self.highest_proposal_id {
                self.highest_proposal_id = promise.last_accepted_proposal_id.clone();
                if promise.last_accepted_value.is_some() {
                    self.value = promise.last_accepted_value.clone();
                }
            }
            if self.received_promises.len() >= self.majority_size {
                return Ok(Message::Propose(ProposeMessage {
                    proposer_id: self.proposer_id.clone(),
                    proposal_id: self.proposal_id.clone(),
                    value: self.value.clone().ok_or_else(|| "value is not set")?
                }))
            }
        }
        Ok(Message::None)
    }

    pub fn set_value(&mut self, instance_id: InstanceID, value: T) -> Result<()> {
        self.value = Some(value);
        Ok(())
    }
}
