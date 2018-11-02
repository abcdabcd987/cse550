use super::common::*;
use errors::*;
use std::collections::HashMap;

struct Learner<T> {
    learner_id: NodeID,
    majority_size: usize,
    proposal_accept_count: HashMap<ProposalID, usize>,
    acceptor_highest_proposal_id: HashMap<NodeID, ProposalID>,
    chosen_proposal_id: ProposalID,
    chosen_value: Option<T>,
}

impl<T: Clone> Learner<T> {
    pub fn new(learner_id: NodeID, cluster_size: usize) -> Learner<T> {
        let chosen_proposal_id = ProposalID::new(0, learner_id.clone());
        Learner {
            learner_id,
            majority_size: (cluster_size + 1) / 2,
            proposal_accept_count: HashMap::new(),
            acceptor_highest_proposal_id: HashMap::new(),
            chosen_proposal_id,
            chosen_value: None
        }
    }

    pub fn receive_accepted(&mut self, accepted: &AcceptedMessage) {
        if self.chosen_value.is_some() {
            return;
        }
        if let Some(id) = self.acceptor_highest_proposal_id.get(&accepted.acceptor_id) {
            if *id >= accepted.proposal_id {
                return;
            }
        }
        self.acceptor_highest_proposal_id.insert(accepted.acceptor_id.clone(), accepted.proposal_id.clone());
        let count = self.proposal_accept_count.entry(accepted.proposal_id.clone()).or_insert(0);
        *count += 1;
        if *count >= self.majority_size {
            self.chosen_proposal_id = accepted.proposal_id.clone();
        }
    }

    pub fn learn_value(&mut self) -> Result<LearnValueMessage> {
        if self.chosen_proposal_id.round() == 0 {
            Err("has not reached consensus yet".into())
        } else {
            Ok(LearnValueMessage {
                learner_id: self.learner_id.clone(),
                proposal_id: self.chosen_proposal_id.clone()
            })
        }
    }

    pub fn receive_value(&mut self, value: &ValueMessage<T>) {
        self.chosen_value = Some(value.value.clone());
    }
}
