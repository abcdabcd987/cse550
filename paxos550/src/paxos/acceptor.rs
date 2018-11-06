use super::common::*;

pub struct Acceptor<T> {
    _instance_id: InstanceID,
    acceptor_id: NodeID,
    highest_promised_proposal_id: ProposalID,
    highest_accepted_proposal_id: ProposalID,
    value: Option<T>,
}

impl<T: Clone> Acceptor<T> {
    pub fn new(instance_id: InstanceID, acceptor_id: NodeID) -> Acceptor<T> {
        let highest_accepted_proposal_id = ProposalID::new(0, acceptor_id.clone());
        Acceptor {
            _instance_id: instance_id,
            acceptor_id,
            highest_promised_proposal_id: highest_accepted_proposal_id.clone(),
            highest_accepted_proposal_id,
            value: None
        }
    }

    pub fn receive_prepare(&mut self, prepare: &PrepareMessage) -> Option<PromiseMessage<T>> {
        if prepare.proposal_id >= self.highest_promised_proposal_id {
            self.highest_promised_proposal_id = prepare.proposal_id.clone();
            Some(PromiseMessage {
                acceptor_id: self.acceptor_id.clone(),
                proposal_id: prepare.proposal_id.clone(),
                last_accepted_proposal_id: self.highest_accepted_proposal_id.clone(),
                last_accepted_value: self.value.clone()
            })
        } else {
            None
        }
    }

    pub fn receive_propose(&mut self, propose: &ProposeMessage<T>) -> Option<AcceptedMessage> {
        if propose.proposal_id >= self.highest_promised_proposal_id {
            self.highest_promised_proposal_id = propose.proposal_id.clone();
            self.highest_accepted_proposal_id = propose.proposal_id.clone();
            self.value = Some(propose.value.clone());
            Some(AcceptedMessage {
                acceptor_id: self.acceptor_id.clone(),
                proposal_id: propose.proposal_id.clone()
            })
        } else {
            None
        }
    }

    pub fn value(&self) -> Option<T> {  // FIXME should be Option<&T>
        self.value.clone()
    }

    pub fn highest_accepted_proposal_id(&self) -> ProposalID {
        self.highest_accepted_proposal_id.clone()
    }
}
