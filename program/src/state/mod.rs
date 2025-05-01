
pub mod authorized;
pub mod authorized_checked_with_seed;
pub mod delegation;
pub mod lockup;
pub mod meta;
pub mod my_state;
pub mod stake;
pub mod stake_flags;
pub mod stake_state_v2;
pub mod utils;

pub use authorized::*;
pub use delegation::*;
pub use lockup::*;
pub use meta::*;
pub use my_state::*;
pub use authorized_checked_with_seed::*;
use pinocchio::{
    account_info::{AccountInfo, Ref},
    program_error::ProgramError,
    ProgramResult,
};
pub use stake::*;
pub use stake_flags::*;
pub use stake_state_v2::*;
pub use utils::*;

pub type Epoch = [u8; 8]; //u64

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

pub fn set_stake_state(
    _stake_account_info: &AccountInfo,
    _new_state: &StakeStateV2,
) -> ProgramResult {
    todo!()
}
