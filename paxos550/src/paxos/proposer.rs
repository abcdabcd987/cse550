use super::common::*;
use errors::*;
use std::vec::Vec;
use std::collections::HashSet;

pub struct Proposer<T> {
    proposer_id: NodeID,
    cluster_size: usize,
    majority_size: usize,
    instances: Vec<ProposerInstance<T>>,
}

struct ProposerInstance<T> {
    proposal_id: ProposalID,
    highest_proposal_id: ProposalID,
    received_promises: HashSet<NodeID>,
    value: Option<T>,
}

impl<T: Clone> Proposer<T> {
    fn get_instance_mut(&mut self, instance_id: InstanceID) -> Result<&mut ProposerInstance<T>> {
        self.instances.get_mut(instance_id)
            .ok_or_else(|| ErrorKind::InstanceNotExists(instance_id).into())
    }

    pub fn new_instance(&mut self) -> InstanceID {
        self.instances.push(ProposerInstance {
            proposal_id: ProposalID::new(0, self.proposer_id.clone()),
            highest_proposal_id: ProposalID::new(0, self.proposer_id.clone()),
            received_promises: HashSet::new(),
            value: None
        });
        self.instances.len() as InstanceID
    }

    pub fn observe_proposal(&mut self, instance_id: InstanceID, proposal_id: &ProposalID) -> Result<()> {
        let instance = self.get_instance_mut(instance_id)?;
        if *proposal_id > instance.highest_proposal_id {
            instance.highest_proposal_id = proposal_id.clone();
        }
        Ok(())
    }

    pub fn prepare(&mut self, instance_id: InstanceID) -> Result<PrepareMessage> {
        let proposer_id = self.proposer_id.clone();
        let instance = self.get_instance_mut(instance_id)?;
        instance.proposal_id = ProposalID::new(instance.highest_proposal_id.round() + 1,
                                               proposer_id.clone());
        instance.highest_proposal_id = instance.proposal_id.clone();
        instance.received_promises.clear();
        Ok(PrepareMessage {
            instance_id,
            proposer_id,
            proposal_id: instance.proposal_id.clone()
        })
    }

    pub fn receive_promise(&mut self, promise: &PromiseMessage<T>) -> Result<Message<T>> {
        let majority_size = self.majority_size;
        let proposer_id = self.proposer_id.clone();
        self.observe_proposal(promise.instance_id, &promise.proposal_id)?;
        let instance = self.get_instance_mut(promise.instance_id)?;
        if instance.proposal_id == promise.proposal_id && !instance.received_promises.contains(&promise.acceptor_id) {
            instance.received_promises.insert(promise.acceptor_id.clone());
            if promise.last_accepted_proposal_id > instance.highest_proposal_id {
                instance.highest_proposal_id = promise.last_accepted_proposal_id.clone();
                if promise.last_accepted_value.is_some() {
                    instance.value = promise.last_accepted_value.clone();
                }
            }
            if instance.received_promises.len() > majority_size {
                return Ok(Message::Propose(ProposeMessage {
                    instance_id: promise.instance_id,
                    proposer_id,
                    proposal_id: instance.proposal_id.clone(),
                    value: instance.value.clone().ok_or_else(|| "value is not set")?
                }))
            }
        }
        Ok(Message::None)
    }

    pub fn set_value(&mut self, instance_id: InstanceID, value: T) -> Result<()> {
        let instance = self.get_instance_mut(instance_id)?;
        instance.value = Some(value);
        Ok(())
    }
}
