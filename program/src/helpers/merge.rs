use {
    super::checked_add,
    crate::{
        consts::PERPETUAL_NEW_WARMUP_COOLDOWN_RATE_EPOCH,
        error::StakeError,
        state::{Clock, Delegation, Meta, Stake, StakeFlags, StakeHistoryGetEntry, StakeStateV2},
    },
    pinocchio::{msg, program_error::ProgramError, sysvars::clock::Epoch, ProgramResult},
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MergeKind {
    Inactive(Meta, u64, StakeFlags),
    ActivationEpoch(Meta, Stake, StakeFlags),
    FullyActive(Meta, Stake),
}

impl MergeKind {
    pub(crate) fn meta(&self) -> &Meta {
        match self {
            Self::Inactive(meta, _, _) => meta,
            Self::ActivationEpoch(meta, _, _) => meta,
            Self::FullyActive(meta, _) => meta,
        }
    }

    pub(crate) fn active_stake(&self) -> Option<&Stake> {
        match self {
            Self::Inactive(_, _, _) => None,
            Self::ActivationEpoch(_, stake, _) => Some(stake),
            Self::FullyActive(_, stake) => Some(stake),
        }
    }

    pub(crate) fn get_if_mergeable<T: StakeHistoryGetEntry>(
        stake_state: &StakeStateV2,
        stake_lamports: u64,
        clock: &Clock,
        stake_history: &T,
    ) -> Result<Self, ProgramError> {
        match stake_state {
            StakeStateV2::Stake(meta, stake, stake_flags) => {
                // stake must not be in a transient state. Transient here meaning
                // activating or deactivating with non-zero effective stake.
                let status = stake.delegation.stake_activating_and_deactivating(
                    clock.epoch,
                    stake_history,
                    PERPETUAL_NEW_WARMUP_COOLDOWN_RATE_EPOCH,
                );

                let effective = u64::from_le_bytes(status.effective);
                let activating = u64::from_le_bytes(status.activating);
                let deactivating = u64::from_le_bytes(status.deactivating);

                match (effective, activating, deactivating) {
                    (0, 0, 0) => Ok(Self::Inactive(*meta, stake_lamports, *stake_flags)),
                    (0, _, _) => Ok(Self::ActivationEpoch(*meta, *stake, *stake_flags)),
                    (_, 0, 0) => Ok(Self::FullyActive(*meta, *stake)),
                    _ => {
                        let err = StakeError::MergeTransientStake;
                        Err(err.into())
                    }
                }
            }
            StakeStateV2::Initialized(meta) => {
                Ok(Self::Inactive(*meta, stake_lamports, StakeFlags::empty()))
            }
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    pub(crate) fn metas_can_merge(stake: &Meta, source: &Meta, clock: &Clock) -> ProgramResult {
        // lockups may mismatch so long as both have expired
        let can_merge_lockups = stake.lockup == source.lockup
            || (!stake.lockup.is_in_force(clock, None) && !source.lockup.is_in_force(clock, None));
        // `rent_exempt_reserve` has no bearing on the mergeability of accounts,
        // as the source account will be culled by runtime once the operation
        // succeeds. Considering it here would needlessly prevent merging stake
        // accounts with differing data lengths, which already exist in the wild
        // due to an SDK bug
        if stake.authorized == source.authorized && can_merge_lockups {
            Ok(())
        } else {
            msg!("Unable to merge due to metadata mismatch");
            Err(StakeError::MergeMismatch.into())
        }
    }

    pub(crate) fn active_delegations_can_merge(
        stake: &Delegation,
        source: &Delegation,
    ) -> ProgramResult {
        if stake.voter_pubkey != source.voter_pubkey {
            msg!("Unable to merge due to voter mismatch");
            Err(StakeError::MergeMismatch.into())
        } else if u64::from_le_bytes(stake.deactivation_epoch) == Epoch::MAX
            && u64::from_le_bytes(source.deactivation_epoch) == Epoch::MAX
        {
            Ok(())
        } else {
            msg!("Unable to merge due to stake deactivation");
            Err(StakeError::MergeMismatch.into())
        }
    }

    pub(crate) fn merge(
        self,
        source: Self,
        clock: &Clock,
    ) -> Result<Option<StakeStateV2>, ProgramError> {
        Self::metas_can_merge(self.meta(), source.meta(), clock)?;
        self.active_stake()
            .zip(source.active_stake())
            .map(|(stake, source)| {
                Self::active_delegations_can_merge(&stake.delegation, &source.delegation)
            })
            .unwrap_or(Ok(()))?;
        let merged_state = match (self, source) {
            (Self::Inactive(_, _, _), Self::Inactive(_, _, _)) => None,
            (Self::Inactive(_, _, _), Self::ActivationEpoch(_, _, _)) => None,
            (
                Self::ActivationEpoch(meta, mut stake, stake_flags),
                Self::Inactive(_, source_lamports, source_stake_flags),
            ) => {
                stake.delegation.stake =
                    checked_add(u64::from_le_bytes(stake.delegation.stake), source_lamports)?
                        .to_le_bytes();
                Some(StakeStateV2::Stake(
                    meta,
                    stake,
                    stake_flags.union(source_stake_flags),
                ))
            }
            (
                Self::ActivationEpoch(meta, mut stake, stake_flags),
                Self::ActivationEpoch(source_meta, source_stake, source_stake_flags),
            ) => {
                let source_lamports = checked_add(
                    u64::from_le_bytes(source_meta.rent_exempt_reserve),
                    u64::from_le_bytes(source_stake.delegation.stake),
                )?;
                merge_delegation_stake_and_credits_observed(
                    &mut stake,
                    source_lamports,
                    source_stake.credits_observed(),
                )?;
                Some(StakeStateV2::Stake(
                    meta,
                    stake,
                    stake_flags.union(source_stake_flags),
                ))
            }
            (Self::FullyActive(meta, mut stake), Self::FullyActive(_, source_stake)) => {
                // Don't stake the source account's `rent_exempt_reserve` to
                // protect against the magic activation loophole. It will
                // instead be moved into the destination account as extra,
                // withdrawable `lamports`
                merge_delegation_stake_and_credits_observed(
                    &mut stake,
                    u64::from_le_bytes(source_stake.delegation.stake),
                    source_stake.credits_observed(),
                )?;
                Some(StakeStateV2::Stake(meta, stake, StakeFlags::empty()))
            }
            _ => return Err(StakeError::MergeMismatch.into()),
        };
        Ok(merged_state)
    }
}

pub(crate) fn merge_delegation_stake_and_credits_observed(
    stake: &mut Stake,
    absorbed_lamports: u64,
    absorbed_credits_observed: u64,
) -> ProgramResult {
    let credits_observed =
        stake_weighted_credits_observed(stake, absorbed_lamports, absorbed_credits_observed)
            .ok_or(ProgramError::ArithmeticOverflow)?;

    stake.set_credits_observed(credits_observed);

    stake.delegation.stake = checked_add(
        u64::from_le_bytes(stake.delegation.stake),
        absorbed_lamports,
    )?
    .to_le_bytes();
    Ok(())
}

pub(crate) fn stake_weighted_credits_observed(
    stake: &Stake,
    absorbed_lamports: u64,
    absorbed_credits_observed: u64,
) -> Option<u64> {
    if stake.credits_observed() == absorbed_credits_observed {
        Some(stake.credits_observed())
    } else {
        let total_stake = u128::from(
            checked_add(
                u64::from_le_bytes(stake.delegation.stake),
                absorbed_lamports,
            )
            .ok()?,
        );

        let stake_weighted_credits = u128::from(stake.credits_observed())
            .checked_mul(u128::from(u64::from_le_bytes(stake.delegation.stake)))?;
        let absorbed_weighted_credits =
            u128::from(absorbed_credits_observed).checked_mul(u128::from(absorbed_lamports))?;
        // Discard fractional credits as a merge side-effect friction by taking
        // the ceiling, done by adding `denominator - 1` to the numerator.
        let total_weighted_credits = stake_weighted_credits
            .checked_add(absorbed_weighted_credits)?
            .checked_add(total_stake)?
            .checked_sub(1)?;
        u64::try_from(total_weighted_credits.checked_div(total_stake)?).ok()
    }
}
