use super::common::*;
use super::{Acceptor, Learner, Proposer};
use errors::*;
use network::message;

use rand;
use rand::Rng;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::hash::Hash;
use std::time::Duration;

pub struct PaxosInstance<T> {
    node_id: NodeID,
    instance_id: InstanceID,
    timeout: Duration,
    messages_to_send: VecDeque<message::MessageInfo<T>>,

    proposer: Proposer<T>,
    acceptor: Acceptor<T>,
    learner: Learner<T>,
    waiting_reply: HashSet<PaxosInstanceMessage<T>>,
    value: Option<T>, // TODO make the canonical copy only exists once (either in Instance, or the  three component)
}

impl<T: Clone + Hash + Eq + Debug> PaxosInstance<T> {
    pub fn new(
        node_id: NodeID,
        instance_id: InstanceID,
        cluster_size: usize,
        timeout: Duration,
    ) -> PaxosInstance<T> {
        PaxosInstance {
            node_id: node_id.clone(), // FIXME remove clone()
            instance_id,
            timeout,
            messages_to_send: VecDeque::new(),
            proposer: Proposer::new(instance_id, node_id.clone(), cluster_size),
            acceptor: Acceptor::new(instance_id, node_id.clone()),
            learner: Learner::new(instance_id, node_id.clone(), cluster_size),
            waiting_reply: HashSet::new(),
            value: None,
        }
    }

    pub fn collect_messages_to_send(&mut self, collector: &mut VecDeque<message::MessageInfo<T>>) {
        // TODO ugly. fixme.
        collector.append(&mut self.messages_to_send);
    }

    fn send_message(
        &mut self,
        message: PaxosInstanceMessage<T>,
        target: message::MessageTarget,
        timeout: Option<Duration>,
    ) {
        if timeout.is_some() {
            self.waiting_reply.insert(message.clone());
        }
        self.messages_to_send.push_back(message::MessageInfo {
            payload: message::MessagePayload::PaxosMessage(PaxosMessage {
                instance_id: self.instance_id,
                message: message,
            }),
            target,
            timeout,
        });
    }

    fn backoff_timeout(&self, timeout: Duration) -> Duration {
        let old = timeout.as_secs() * 1000 + timeout.subsec_millis() as u64;
        let backoff = rand::thread_rng().gen_range(0, old);
        Duration::from_millis(old + backoff)
    }

    pub fn start_proposing(&mut self, value: T) {
        self.proposer.set_value(value);
        let timeout = self.timeout; // need NLL
        self.do_prepare(timeout);
    }

    pub fn value(&mut self) -> Option<&T> {
        self.value.as_ref()
    }

    //    pub fn start_recovery(&mut self) {
    //        let msg = PaxosInstanceMessage::Recovery(RecoveryMessage {
    //            node_id: self.node_id.clone()
    //        });
    //        let timeout = Some(self.timeout);
    //        self.send_message(msg, message::MessageTarget::Broadcast, timeout);
    //    }

    fn do_prepare(&mut self, timeout: Duration) {
        let msg = PaxosInstanceMessage::Prepare(self.proposer.prepare());
        self.send_message(msg, message::MessageTarget::Broadcast, Some(timeout));
    }

    /// Returns `Some` if this is the first time the learner learns the value.
    pub fn receive_message(&mut self, message: &PaxosInstanceMessage<T>) -> Option<T> {
        // FIXME should return Option<&T>
        match *message {
            PaxosInstanceMessage::Prepare(ref prepare) => {
                self.proposer.observe_proposal(&prepare.proposal_id);
                if let Some(m) = self.acceptor.receive_prepare(prepare) {
                    let msg = PaxosInstanceMessage::Promise(m);
                    let target = message::MessageTarget::Node(prepare.proposer_id.clone());
                    self.send_message(msg, target, None);
                }
            }
            PaxosInstanceMessage::Promise(ref promise) => {
                if let Some(m) = self.proposer.receive_promise(promise) {
                    // if got Promise from the majority, clear the Prepare timeout
                    self.waiting_reply.retain(|msg| match *msg {
                        PaxosInstanceMessage::Prepare(ref prepare) => {
                            m.proposal_id != prepare.proposal_id
                        }
                        _ => true,
                    });

                    let msg = PaxosInstanceMessage::Propose(m);
                    let timeout = Some(self.timeout);
                    self.send_message(msg, message::MessageTarget::Broadcast, timeout);
                }
            }
            PaxosInstanceMessage::Propose(ref propose) => {
                self.proposer.observe_proposal(&propose.proposal_id);
                if let Some(m) = self.acceptor.receive_propose(propose) {
                    let msg = PaxosInstanceMessage::Accepted(m);
                    self.send_message(msg, message::MessageTarget::Broadcast, None);
                }
            }
            PaxosInstanceMessage::Accepted(ref accepted) => {
                self.proposer.observe_proposal(&accepted.proposal_id);
                if let Some(m) = self.learner.receive_accepted(accepted) {
                    // if got Accepted from the majority, clear the Promise timeout
                    self.waiting_reply.retain(|msg| match *msg {
                        PaxosInstanceMessage::Propose(ref propose) => {
                            propose.proposal_id != accepted.proposal_id
                        }
                        _ => true,
                    });

                    if let Some(v) = self.acceptor.value() {
                        // if the acceptor accepted the proposal, directly set the value.
                        self.learner.set_chosen_value(v.clone());
                        self.value = Some(v.clone());
                        self.acceptor.set_reached_consensus();
                        return Some(v.clone());
                    } else {
                        // otherwise, ask other nodes for the answer.
                        let msg = PaxosInstanceMessage::Learn(m);
                        let timeout = Some(self.timeout);
                        self.send_message(msg, message::MessageTarget::Broadcast, timeout);
                    }
                }
            }
            PaxosInstanceMessage::Learn(ref learn) => {
                if let Some(m) = self.learner.receive_learn(learn) {
                    let msg = PaxosInstanceMessage::Value(m);
                    self.send_message(
                        msg,
                        message::MessageTarget::Node(learn.learner_id.clone()),
                        None,
                    );
                }
            }
            PaxosInstanceMessage::Value(ref value) => {
                // if got Value from any node, clear all the Learn timeout
                self.waiting_reply.retain(|msg| match *msg {
                    PaxosInstanceMessage::Learn(_) => false,
                    _ => true,
                });

                self.value = Some(value.chosen_value.clone());
                self.acceptor.set_reached_consensus();
                return self.learner.receive_value(value);
            }
            //            PaxosInstanceMessage::Recovery(ref recovery) => {
            //                if let Some(v) = self.value.clone() {  // FIXME clone()
            //                    let msg = PaxosInstanceMessage::Consensus(ConsensusMessage {
            //                        node_id: self.node_id.clone(),
            //                        proposal_id: self.acceptor.highest_accepted_proposal_id(),
            //                        value: v,
            //                    });
            //                    self.send_message(msg, message::MessageTarget::Node(recovery.node_id.clone()), None);
            //                }
            //            },
            //            PaxosInstanceMessage::Consensus(ref consensus) => {
            //                if self.value.is_none() {
            //                    self.value = Some(consensus.value.clone());
            //                    self.proposer.receive_consensus(consensus);
            //                    self.acceptor.receive_consensus(consensus);
            //                    self.learner.receive_consensus(consensus);
            //                    return self.value.clone();
            //                }
            //            }
        }
        None
    }

    pub fn on_timeout(
        &mut self,
        message: PaxosInstanceMessage<T>,
        timeout: Duration,
    ) -> Result<()> {
        if !self.waiting_reply.remove(&message) {
            return Ok(());
        }
        debug!(
            "instance {} timeout {:?} {:?}",
            self.instance_id, timeout, message
        );
        let new_timeout = self.backoff_timeout(timeout);
        match message {
            PaxosInstanceMessage::Prepare(..) |
            PaxosInstanceMessage::Propose(..) => {
                if self.value.is_none() {
                    self.do_prepare(new_timeout);
                }
            },
            PaxosInstanceMessage::Promise(..) |
            PaxosInstanceMessage::Accepted(..) |
//            PaxosInstanceMessage::Consensus(..) |
            PaxosInstanceMessage::Value(..) => {
                return Err("this message shouldn't wait for reply".into());
            },
            PaxosInstanceMessage::Learn(..) => {
                // send the Learn message again
                self.send_message(message, message::MessageTarget::Broadcast, Some(new_timeout));
            },
//            PaxosInstanceMessage::Recovery(..) => {
//                if self.value.is_none() {
//                    // send the Recovery message again
//                    self.send_message(message, message::MessageTarget::Broadcast, Some(new_timeout));
//                }
//            },
        }
        Ok(())
    }
}
