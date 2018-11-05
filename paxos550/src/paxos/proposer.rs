use super::common::*;
use std::collections::HashSet;

pub struct Proposer<T> {
    _instance_id: InstanceID,
    proposer_id: NodeID,
    majority_size: usize,
    proposal_id: ProposalID,
    highest_proposal_id: ProposalID,
    received_promises: HashSet<NodeID>,
    value: Option<T>,
}

impl<T: Clone> Proposer<T> {
    pub fn new(instance_id: InstanceID, proposer_id: NodeID, cluster_size: usize) -> Proposer<T> {
        let highest_proposal_id = ProposalID::new(0, proposer_id.clone());
        Proposer {
            _instance_id: instance_id,
            proposer_id,
            majority_size: (cluster_size + 1) / 2,
            proposal_id: highest_proposal_id.clone(),
            highest_proposal_id,
            received_promises: HashSet::new(),
            value: None
        }
    }

    pub fn observe_proposal(&mut self, proposal_id: &ProposalID) {
        if *proposal_id > self.highest_proposal_id {
            self.highest_proposal_id = proposal_id.clone();
        }
    }

    pub fn prepare(&mut self) -> PrepareMessage {
        self.proposal_id = ProposalID::new(self.highest_proposal_id.round() + 1,
                                           self.proposer_id.clone());
        self.highest_proposal_id = self.proposal_id.clone();
        self.received_promises.clear();
        PrepareMessage {
            proposer_id: self.proposer_id.clone(),
            proposal_id: self.proposal_id.clone()
        }
    }

    pub fn receive_promise(&mut self, promise: &PromiseMessage<T>) -> Option<ProposeMessage<T>> {
        self.observe_proposal(&promise.proposal_id);
        if self.proposal_id == promise.proposal_id && !self.received_promises.contains(&promise.acceptor_id) {
            self.received_promises.insert(promise.acceptor_id.clone());
            if promise.last_accepted_proposal_id > self.highest_proposal_id {
                self.highest_proposal_id = promise.last_accepted_proposal_id.clone();
                if promise.last_accepted_value.is_some() {
                    self.value = promise.last_accepted_value.clone();
                }
            }
            if self.received_promises.len() >= self.majority_size {
                return Some(ProposeMessage {
                    proposer_id: self.proposer_id.clone(),
                    proposal_id: self.proposal_id.clone(),
                    value: self.value.as_ref().expect("assert has value").clone()
                })
            }
        }
        None
    }

    pub fn set_value(&mut self, value: T) {
        self.value = Some(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type T = String;
    fn construct_proposer() -> Proposer<T> {
        Proposer::new(0, String::from("proposer_1"), 2)
    }
}
