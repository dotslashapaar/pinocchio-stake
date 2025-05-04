use crate::error::StakeError;

use super::{bytes_to_u64, Delegation, Epoch, StakeHistoryGetEntry};

#[repr(C)]
#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub struct Stake {
    pub delegation: Delegation,
    /// credits observed is credits from vote account state when delegated or redeemed
    pub credits_observed: [u8; 8], //u64
    // changed to pub (as required in utils.rs L511 and L455)
}

impl Stake {
    #[inline(always)]
    pub fn set_credits_observed(&mut self, credits_observed: u64) {
        self.credits_observed = credits_observed.to_le_bytes();
    }

    #[inline(always)]
    pub fn credits_observed(&self) -> u64 {
        u64::from_le_bytes(self.credits_observed)
    }

    pub fn stake<T: StakeHistoryGetEntry>(
        &self,
        epoch: Epoch,
        history: &T,
        new_rate_activation_epoch: Option<Epoch>,
    ) -> u64 {
        self.delegation
            .stake(epoch, history, new_rate_activation_epoch)
    }

    pub fn split(
        &mut self,
        remaining_stake_delta: u64,
        split_stake_amount: u64,
    ) -> Result<Self, StakeError> {
        if remaining_stake_delta > bytes_to_u64(self.delegation.stake) {
            return Err(StakeError::InsufficientStake);
        }
        self.delegation.stake = bytes_to_u64(self.delegation.stake)
            .saturating_sub(remaining_stake_delta)
            .to_le_bytes();
        let new = Self {
            delegation: Delegation {
                stake: split_stake_amount.to_le_bytes(),
                ..self.delegation
            },
            ..*self
        };
        Ok(new)
    }

    pub fn deactivate(&mut self, epoch: Epoch) -> Result<(), StakeError> {
        if bytes_to_u64(self.delegation.deactivation_epoch) != u64::MAX {
            Err(StakeError::AlreadyDeactivated)
        } else {
            self.delegation.deactivation_epoch = epoch;
            Ok(())
        }
    }
}
