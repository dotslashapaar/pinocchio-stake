pub mod authorized;
pub mod delegation;
pub mod lockup;
pub mod meta;
pub mod redelegate_state;
pub mod stake;
pub mod stake_authorize;
pub mod stake_flags;
pub mod stake_history;
pub mod stake_history_sysvar;
pub mod stake_state_v2;
pub mod utils;

pub use authorized::*;
pub use delegation::*;
pub use lockup::*;
pub use meta::*;
use pinocchio::{
    account_info::{AccountInfo, Ref, RefMut},
    program_error::ProgramError,
    ProgramResult,
};

pub use redelegate_state::*;
pub use stake::*;
pub use stake_authorize::*;
pub use stake_flags::*;
pub use stake_history::*;
pub use stake_history_sysvar::*;
pub use stake_state_v2::*;
pub use utils::*;

pub type Epoch = [u8; 8]; //u64
pub type UnixTimestamp = [u8; 8]; //i64;

pub fn get_stake_state(
    stake_account_info: &AccountInfo,
) -> Result<Ref<StakeStateV2>, ProgramError> {
    if stake_account_info.is_owned_by(&crate::ID) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    StakeStateV2::from_account_info(stake_account_info)
}

/// # Safety
///
/// The caller must ensure that it is safe to borrow the account data – e.g., there are
/// no mutable borrows of the account data.
pub unsafe fn get_stake_state_unchecked(
    stake_account_info: &AccountInfo,
) -> Result<&StakeStateV2, ProgramError> {
    if stake_account_info.owner() != &crate::ID {
        return Err(ProgramError::InvalidAccountOwner);
    }

    StakeStateV2::from_account_info_unchecked(stake_account_info)
}

pub fn try_get_stake_state_mut(
    stake_account_info: &AccountInfo,
) -> Result<RefMut<StakeStateV2>, ProgramError> {
    if stake_account_info.is_owned_by(&crate::ID) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    StakeStateV2::try_from_account_info_mut(stake_account_info)
}

// dont call this "move" because we have an instruction MoveLamports
pub fn relocate_lamports(
    source_account_info: &AccountInfo,
    destination_account_info: &AccountInfo,
    lamports: u64,
) -> ProgramResult {
    {
        let mut source_lamports = source_account_info.try_borrow_mut_lamports()?;
        *source_lamports = source_lamports
            .checked_sub(lamports)
            .ok_or(ProgramError::InsufficientFunds)?;
    }

    {
        let mut destination_lamports = destination_account_info.try_borrow_mut_lamports()?;
        *destination_lamports = destination_lamports
            .checked_add(lamports)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    Ok(())
}
