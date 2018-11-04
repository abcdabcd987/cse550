use std::collections::VecDeque;
use super::{Proposer, Acceptor, Learner};
use super::common::*;

pub struct PaxosInstance<T> {
    node_id: NodeID,
    cluster_size: usize,
    proposer: Proposer<T>,
    acceptor: Acceptor<T>,
    learner: Learner<T>,
    messages_to_send: VecDeque<Message<T>>,
}

impl<T: Clone> PaxosInstance<T> {
    pub fn new(node_id: &NodeID, cluster_size: usize) -> PaxosInstance<T> {
        PaxosInstance {
            node_id: node_id.clone(),
            cluster_size,
            proposer: Proposer::new(node_id.clone(), cluster_size),
            acceptor: Acceptor::new(node_id.clone()),
            learner: Learner::new(node_id.clone(), cluster_size),
            messages_to_send: VecDeque::new()
        }
    }

    pub fn start_proposing(&mut self, value: T) {
        self.proposer.set_value(value);
        self.messages_to_send.push_back(Message {
            target: MessageTarget::Broadcast,
            payload: MessagePayload::Prepare(self.proposer.prepare())
        });
    }

    pub fn receive_message(&mut self, message: &MessagePayload<T>) {
        match *message {
            MessagePayload::Prepare(ref prepare) => {
                self.proposer.observe_proposal(&prepare.proposal_id);
                if let Some(m) = self.acceptor.receive_prepare(prepare) {
                    self.messages_to_send.push_back(Message {
                        target: MessageTarget::Node(prepare.proposer_id.clone()),
                        payload: MessagePayload::Promise(m)
                    });
                }
            },
            MessagePayload::Promise(ref promise) => {
                if let Some(m) = self.proposer.receive_promise(promise) {
                    self.messages_to_send.push_back(Message {
                        target: MessageTarget::Broadcast,
                        payload: MessagePayload::Propose(m)
                    });
                }
            },
            MessagePayload::Propose(ref propose) => {
                self.proposer.observe_proposal(&propose.proposal_id);
                if let Some(m) = self.acceptor.receive_propose(propose) {
                    self.messages_to_send.push_back(Message {
                        target: MessageTarget::Broadcast,
                        payload: MessagePayload::Accepted(m)
                    });
                }
            },
            MessagePayload::Accepted(ref accepted) => {
                self.learner.receive_accepted(accepted);
            },
            MessagePayload::Learn(ref learn) => {
                if let Some(m) = self.learner.receive_learn(learn) {
                    self.messages_to_send.push_back(Message {
                        target: MessageTarget::Broadcast,
                        payload: MessagePayload::Value(m)
                    });
                }
            },
            MessagePayload::Value(ref value) => {
                self.learner.receive_value(value);
            },
        }
    }
}
