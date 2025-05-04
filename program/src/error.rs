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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum InstructionError {
    /// Deprecated! Use CustomError instead!
    /// The program instruction returned an error
    GenericError,

    /// The arguments provided to a program were invalid
    InvalidArgument,

    /// An instruction's data contents were invalid
    InvalidInstructionData,

    /// An account's data contents was invalid
    InvalidAccountData,

    /// An account's data was too small
    AccountDataTooSmall,

    /// An account's balance was too small to complete the instruction
    InsufficientFunds,

    /// The account did not have the expected program id
    IncorrectProgramId,

    /// A signature was required but not found
    MissingRequiredSignature,

    /// An initialize instruction was sent to an account that has already been initialized.
    AccountAlreadyInitialized,

    /// An attempt to operate on an account that hasn't been initialized.
    UninitializedAccount,

    /// Program's instruction lamport balance does not equal the balance after the instruction
    UnbalancedInstruction,

    /// Program illegally modified an account's program id
    ModifiedProgramId,

    /// Program spent the lamports of an account that doesn't belong to it
    ExternalAccountLamportSpend,

    /// Program modified the data of an account that doesn't belong to it
    ExternalAccountDataModified,

    /// Read-only account's lamports modified
    ReadonlyLamportChange,

    /// Read-only account's data was modified
    ReadonlyDataModified,

    /// An account was referenced more than once in a single instruction
    // Deprecated, instructions can now contain duplicate accounts
    DuplicateAccountIndex,

    /// Executable bit on account changed, but shouldn't have
    ExecutableModified,

    /// Rent_epoch account changed, but shouldn't have
    RentEpochModified,

    /// The instruction expected additional account keys
    NotEnoughAccountKeys,

    /// Program other than the account's owner changed the size of the account data
    AccountDataSizeChanged,

    /// The instruction expected an executable account
    AccountNotExecutable,

    /// Failed to borrow a reference to account data, already borrowed
    AccountBorrowFailed,

    /// Account data has an outstanding reference after a program's execution
    AccountBorrowOutstanding,

    /// The same account was multiply passed to an on-chain program's entrypoint, but the program
    /// modified them differently.  A program can only modify one instance of the account because
    /// the runtime cannot determine which changes to pick or how to merge them if both are modified
    DuplicateAccountOutOfSync,

    /// Allows on-chain programs to implement program-specific error types and see them returned
    /// by the Solana runtime. A program-specific error may be any type that is represented as
    /// or serialized to a u32 integer.
    Custom(u32),

    /// The return value from the program was invalid.  Valid errors are either a defined builtin
    /// error value or a user-defined error in the lower 32 bits.
    InvalidError,

    /// Executable account's data was modified
    ExecutableDataModified,

    /// Executable account's lamports modified
    ExecutableLamportChange,

    /// Executable accounts must be rent exempt
    ExecutableAccountNotRentExempt,

    /// Unsupported program id
    UnsupportedProgramId,

    /// Cross-program invocation call depth too deep
    CallDepth,

    /// An account required by the instruction is missing
    MissingAccount,

    /// Cross-program invocation reentrancy not allowed for this instruction
    ReentrancyNotAllowed,

    /// Length of the seed is too long for address generation
    MaxSeedLengthExceeded,

    /// Provided seeds do not result in a valid address
    InvalidSeeds,

    /// Failed to reallocate account data of this length
    InvalidRealloc,

    /// Computational budget exceeded
    ComputationalBudgetExceeded,

    /// Cross-program invocation with unauthorized signer or writable account
    PrivilegeEscalation,

    /// Failed to create program execution environment
    ProgramEnvironmentSetupFailure,

    /// Program failed to complete
    ProgramFailedToComplete,

    /// Program failed to compile
    ProgramFailedToCompile,

    /// Account is immutable
    Immutable,

    /// Incorrect authority provided
    IncorrectAuthority,

    /// Failed to serialize or deserialize account data
    ///
    /// Warning: This error should never be emitted by the runtime.
    ///
    /// This error includes strings from the underlying 3rd party Borsh crate
    /// which can be dangerous because the error strings could change across
    /// Borsh versions. Only programs can use this error because they are
    /// consistent across Solana software versions.
    ///
    // BorshIoError(String),

    /// An account does not have enough lamports to be rent-exempt
    AccountNotRentExempt,

    /// Invalid account owner
    InvalidAccountOwner,

    /// Program arithmetic overflowed
    ArithmeticOverflow,

    /// Unsupported sysvar
    UnsupportedSysvar,

    /// Illegal account owner
    IllegalOwner,

    /// Accounts data allocations exceeded the maximum allowed per transaction
    MaxAccountsDataAllocationsExceeded,

    /// Max accounts exceeded
    MaxAccountsExceeded,

    /// Max instruction trace length exceeded
    MaxInstructionTraceLengthExceeded,

    /// Builtin programs must consume compute units
    BuiltinProgramsMustConsumeComputeUnits,
    // Note: For any new error added here an equivalent ProgramError and its
    // conversions must also be added
}

impl TryFrom<InstructionError> for ProgramError {
    type Error = InstructionError;

    fn try_from(error: InstructionError) -> Result<Self, Self::Error> {
        match error {
            Self::Error::Custom(err) => Ok(Self::Custom(err)),
            Self::Error::InvalidArgument => Ok(Self::InvalidArgument),
            Self::Error::InvalidInstructionData => Ok(Self::InvalidInstructionData),
            Self::Error::InvalidAccountData => Ok(Self::InvalidAccountData),
            Self::Error::AccountDataTooSmall => Ok(Self::AccountDataTooSmall),
            Self::Error::InsufficientFunds => Ok(Self::InsufficientFunds),
            Self::Error::IncorrectProgramId => Ok(Self::IncorrectProgramId),
            Self::Error::MissingRequiredSignature => Ok(Self::MissingRequiredSignature),
            Self::Error::AccountAlreadyInitialized => Ok(Self::AccountAlreadyInitialized),
            Self::Error::UninitializedAccount => Ok(Self::UninitializedAccount),
            Self::Error::NotEnoughAccountKeys => Ok(Self::NotEnoughAccountKeys),
            Self::Error::AccountBorrowFailed => Ok(Self::AccountBorrowFailed),
            Self::Error::MaxSeedLengthExceeded => Ok(Self::MaxSeedLengthExceeded),
            Self::Error::InvalidSeeds => Ok(Self::InvalidSeeds),
            // Self::Error::BorshIoError(err) => Ok(Self::BorshIoError(err)),
            Self::Error::AccountNotRentExempt => Ok(Self::AccountNotRentExempt),
            Self::Error::UnsupportedSysvar => Ok(Self::UnsupportedSysvar),
            Self::Error::IllegalOwner => Ok(Self::IllegalOwner),
            Self::Error::MaxAccountsDataAllocationsExceeded => {
                Ok(Self::MaxAccountsDataAllocationsExceeded)
            }
            Self::Error::InvalidRealloc => Ok(Self::InvalidRealloc),
            Self::Error::MaxInstructionTraceLengthExceeded => {
                Ok(Self::MaxInstructionTraceLengthExceeded)
            }
            Self::Error::BuiltinProgramsMustConsumeComputeUnits => {
                Ok(Self::BuiltinProgramsMustConsumeComputeUnits)
            }
            Self::Error::InvalidAccountOwner => Ok(Self::InvalidAccountOwner),
            Self::Error::ArithmeticOverflow => Ok(Self::ArithmeticOverflow),
            Self::Error::Immutable => Ok(Self::Immutable),
            Self::Error::IncorrectAuthority => Ok(Self::IncorrectAuthority),
            _ => Err(error),
        }
    }
}

pub(crate) fn to_program_error(e: InstructionError) -> ProgramError {
    ProgramError::try_from(e).unwrap_or(ProgramError::InvalidAccountData)
}
