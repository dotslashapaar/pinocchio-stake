use core::fmt;

use pinocchio::program_error::ProgramError;

pub trait FromPrimitive {
    fn from_u64(n: u64) -> Option<Self>
    where
        Self: Sized;
    fn from_i64(n: i64) -> Option<Self>
    where
        Self: Sized;
}

pub trait ToPrimitive {
    fn to_i64(&self) -> Option<i64>;
    fn to_u64(&self) -> Option<u64> {
        self.to_i64().map(|v| v as u64)
    }
}

/// Reasons the Stake might have had an error.
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Deserialize, serde_derive::Serialize)
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StakeError {
    // 0
    /// Not enough credits to redeem.
    NoCreditsToRedeem,

    /// Lockup has not yet expired.
    LockupInForce,

    /// Stake already deactivated.
    AlreadyDeactivated,

    /// One re-delegation permitted per epoch.
    TooSoonToRedelegate,

    /// Split amount is more than is staked.
    InsufficientStake,

    // 5
    /// Stake account with transient stake cannot be merged.
    MergeTransientStake,

    /// Stake account merge failed due to different authority, lockups or state.
    MergeMismatch,

    /// Custodian address not present.
    CustodianMissing,

    /// Custodian signature not present.
    CustodianSignatureMissing,

    /// Insufficient voting activity in the reference vote account.
    InsufficientReferenceVotes,

    // 10
    /// Stake account is not delegated to the provided vote account.
    VoteAddressMismatch,

    /// Stake account has not been delinquent for the minimum epochs required
    /// for deactivation.
    MinimumDelinquentEpochsForDeactivationNotMet,

    /// Delegation amount is less than the minimum.
    InsufficientDelegation,

    /// Stake account with transient or inactive stake cannot be redelegated.
    RedelegateTransientOrInactiveStake,

    /// Stake redelegation to the same vote account is not permitted.
    RedelegateToSameVoteAccount,

    // 15
    /// Redelegated stake must be fully activated before deactivation.
    RedelegatedStakeMustFullyActivateBeforeDeactivationIsPermitted,

    /// Stake action is not permitted while the epoch rewards period is active.
    EpochRewardsActive,
}

impl From<StakeError> for ProgramError {
    fn from(e: StakeError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl FromPrimitive for StakeError {
    #[inline]
    fn from_i64(n: i64) -> Option<Self> {
        if n == Self::NoCreditsToRedeem as i64 {
            Some(Self::NoCreditsToRedeem)
        } else if n == Self::LockupInForce as i64 {
            Some(Self::LockupInForce)
        } else if n == Self::AlreadyDeactivated as i64 {
            Some(Self::AlreadyDeactivated)
        } else if n == Self::TooSoonToRedelegate as i64 {
            Some(Self::TooSoonToRedelegate)
        } else if n == Self::InsufficientStake as i64 {
            Some(Self::InsufficientStake)
        } else if n == Self::MergeTransientStake as i64 {
            Some(Self::MergeTransientStake)
        } else if n == Self::MergeMismatch as i64 {
            Some(Self::MergeMismatch)
        } else if n == Self::CustodianMissing as i64 {
            Some(Self::CustodianMissing)
        } else if n == Self::CustodianSignatureMissing as i64 {
            Some(Self::CustodianSignatureMissing)
        } else if n == Self::InsufficientReferenceVotes as i64 {
            Some(Self::InsufficientReferenceVotes)
        } else if n == Self::VoteAddressMismatch as i64 {
            Some(Self::VoteAddressMismatch)
        } else if n == Self::MinimumDelinquentEpochsForDeactivationNotMet as i64 {
            Some(Self::MinimumDelinquentEpochsForDeactivationNotMet)
        } else if n == Self::InsufficientDelegation as i64 {
            Some(Self::InsufficientDelegation)
        } else if n == Self::RedelegateTransientOrInactiveStake as i64 {
            Some(Self::RedelegateTransientOrInactiveStake)
        } else if n == Self::RedelegateToSameVoteAccount as i64 {
            Some(Self::RedelegateToSameVoteAccount)
        } else if n == Self::RedelegatedStakeMustFullyActivateBeforeDeactivationIsPermitted as i64 {
            Some(Self::RedelegatedStakeMustFullyActivateBeforeDeactivationIsPermitted)
        } else if n == Self::EpochRewardsActive as i64 {
            Some(Self::EpochRewardsActive)
        } else {
            None
        }
    }
    #[inline]
    fn from_u64(n: u64) -> Option<Self> {
        Self::from_i64(n as i64)
    }
}

impl ToPrimitive for StakeError {
    #[inline]
    fn to_i64(&self) -> Option<i64> {
        Some(match *self {
            Self::NoCreditsToRedeem => Self::NoCreditsToRedeem as i64,
            Self::LockupInForce => Self::LockupInForce as i64,
            Self::AlreadyDeactivated => Self::AlreadyDeactivated as i64,
            Self::TooSoonToRedelegate => Self::TooSoonToRedelegate as i64,
            Self::InsufficientStake => Self::InsufficientStake as i64,
            Self::MergeTransientStake => Self::MergeTransientStake as i64,
            Self::MergeMismatch => Self::MergeMismatch as i64,
            Self::CustodianMissing => Self::CustodianMissing as i64,
            Self::CustodianSignatureMissing => Self::CustodianSignatureMissing as i64,
            Self::InsufficientReferenceVotes => Self::InsufficientReferenceVotes as i64,
            Self::VoteAddressMismatch => Self::VoteAddressMismatch as i64,
            Self::MinimumDelinquentEpochsForDeactivationNotMet => {
                Self::MinimumDelinquentEpochsForDeactivationNotMet as i64
            }
            Self::InsufficientDelegation => Self::InsufficientDelegation as i64,
            Self::RedelegateTransientOrInactiveStake => {
                Self::RedelegateTransientOrInactiveStake as i64
            }
            Self::RedelegateToSameVoteAccount => Self::RedelegateToSameVoteAccount as i64,
            Self::RedelegatedStakeMustFullyActivateBeforeDeactivationIsPermitted => {
                Self::RedelegatedStakeMustFullyActivateBeforeDeactivationIsPermitted as i64
            }
            Self::EpochRewardsActive => Self::EpochRewardsActive as i64,
        })
    }
    #[inline]
    fn to_u64(&self) -> Option<u64> {
        self.to_i64().map(|x| x as u64)
    }
}
