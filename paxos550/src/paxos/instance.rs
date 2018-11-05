use super::{Proposer, Acceptor, Learner};
use super::common::*;
use errors::*;
use network::message;

use rand;
use rand::Rng;
use std::collections::VecDeque;
use std::time::Duration;
use std::collections::HashSet;
use std::hash::Hash;
use std::cell::RefCell;

pub struct PaxosInstance<T> {
    node_id: NodeID,
    instance_id: InstanceID,
    cluster_size: usize,
    timeout: Duration,
    messages_to_send: VecDeque<message::MessageInfo<T>>,

    proposer: Proposer<T>,
    acceptor: Acceptor<T>,
    learner: Learner<T>,
    waiting_reply: HashSet<PaxosInstanceMessage<T>>,
}

impl<T: Clone + Hash + Eq> PaxosInstance<T> {
    pub fn new(node_id: &NodeID, instance_id: InstanceID, cluster_size: usize, timeout: Duration) -> PaxosInstance<T> {
        PaxosInstance {
            node_id: node_id.clone(),
            instance_id,
            cluster_size,
            timeout,
            messages_to_send: VecDeque::new(),
            proposer: Proposer::new(instance_id, node_id.clone(), cluster_size),
            acceptor: Acceptor::new(instance_id, node_id.clone()),
            learner: Learner::new(instance_id, node_id.clone(), cluster_size),
            waiting_reply: HashSet::new()
        }
    }

    pub fn collect_messages_to_send(&mut self, global: &mut VecDeque<message::MessageInfo<T>>) {
        // TODO ugly. fixme.
        global.append(&mut self.messages_to_send);
    }

    fn send_message(&mut self, message: PaxosInstanceMessage<T>, target: message::MessageTarget,
                    timeout: Option<Duration>)
    {
        if timeout.is_some() {
            self.waiting_reply.insert(message.clone());
        }
        self.messages_to_send.push_back(message::MessageInfo {
            payload: message::MessagePayload::PaxosMessage(PaxosMessage {
                instance_id: self.instance_id,
                message: message
            }),
            target,
            timeout
        });
    }

    fn backoff_timeout(&self, timeout: Duration) -> Duration {
        let old = timeout.as_secs() * 1000 + timeout.subsec_millis() as u64;
        let backoff = rand::thread_rng().gen_range(0, old);
        Duration::from_millis(old + backoff)
    }

    pub fn start_proposing(&mut self, value: T) {
        self.proposer.set_value(value);
        let timeout = self.timeout;  // need NLL
        self.do_prepare(timeout);
    }

    fn do_prepare(&mut self, timeout: Duration) {
        let msg = PaxosInstanceMessage::Prepare(self.proposer.prepare());
        self.send_message(msg, message::MessageTarget::Broadcast, Some(timeout));
    }

    pub fn receive_message(&mut self, message: &PaxosInstanceMessage<T>) {
        match *message {
            PaxosInstanceMessage::Prepare(ref prepare) => {
                self.proposer.observe_proposal(&prepare.proposal_id);
                if let Some(m) = self.acceptor.receive_prepare(prepare) {
                    let msg = PaxosInstanceMessage::Promise(m);
                    let target = message::MessageTarget::Node(prepare.proposer_id.clone());
                    self.send_message(msg, target, None);
                }
            },
            PaxosInstanceMessage::Promise(ref promise) => {
                if let Some(m) = self.proposer.receive_promise(promise) {
                    // if got Promise from the majority, clear the Prepare timeout
                    self.waiting_reply.retain(|msg| match *msg {
                        PaxosInstanceMessage::Prepare(ref prepare) =>
                            m.proposal_id != prepare.proposal_id,
                        _ => true
                    });

                    let msg = PaxosInstanceMessage::Propose(m);
                    let timeout = Some(self.timeout);
                    self.send_message(msg, message::MessageTarget::Broadcast, timeout);
                }
            },
            PaxosInstanceMessage::Propose(ref propose) => {
                self.proposer.observe_proposal(&propose.proposal_id);
                if let Some(m) = self.acceptor.receive_propose(propose) {
                    let msg = PaxosInstanceMessage::Accepted(m);
                    self.send_message(msg, message::MessageTarget::Broadcast, None);
                }
            },
            PaxosInstanceMessage::Accepted(ref accepted) => {
                self.proposer.observe_proposal(&accepted.proposal_id);
                if let Some(m) = self.learner.receive_accepted(accepted) {
                    // if got Accepted from the majority, clear the Promise timeout
                    self.waiting_reply.retain(|msg| match *msg {
                        PaxosInstanceMessage::Promise(ref promise) =>
                            promise.proposal_id != accepted.proposal_id,
                        _ => true
                    });

                    let msg = PaxosInstanceMessage::Learn(m);
                    let timeout = Some(self.timeout);
                    self.send_message(msg, message::MessageTarget::Broadcast, timeout);
                }
            },
            PaxosInstanceMessage::Learn(ref learn) => {
                if let Some(m) = self.learner.receive_learn(learn) {
                    let msg = PaxosInstanceMessage::Value(m);
                    let timeout = Some(self.timeout);
                    self.send_message(msg, message::MessageTarget::Broadcast, timeout);
                }
            },
            PaxosInstanceMessage::Value(ref value) => {
                // if got Value from any node, clear all the Learn timeout
                self.waiting_reply.retain(|msg| match *msg {
                    PaxosInstanceMessage::Learn(_) => false,
                    _ => true
                });

                self.learner.receive_value(value);
            },
        }
    }

    pub fn on_timeout(&mut self, message: PaxosInstanceMessage<T>, timeout: Duration) -> Result<()> {
        if !self.waiting_reply.remove(&message) {
            return Ok(());
        }
        let new_timeout = self.backoff_timeout(timeout);
        match message {
            PaxosInstanceMessage::Prepare(..) |
            PaxosInstanceMessage::Propose(..) => {
                self.do_prepare(new_timeout);
            },
            PaxosInstanceMessage::Promise(..) |
            PaxosInstanceMessage::Accepted(..) |
            PaxosInstanceMessage::Value(_) => {
                return Err("this message shouldn't wait for reply".into());
            },
            PaxosInstanceMessage::Learn(_) => {
                // send the Learn message again
                self.send_message(message, message::MessageTarget::Broadcast, Some(new_timeout));
            },
        }
        Ok(())
    }
}
