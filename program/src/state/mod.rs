pub mod authorized;
pub mod delegation;
pub mod lockup;
pub mod meta;
pub mod stake;
pub mod stake_authorize;
pub mod stake_clock;
pub mod stake_flags;
pub mod stake_history;
pub mod stake_history_sysvar;
pub mod stake_state_v2;
pub mod utils;
pub mod redelegate_state;

pub use authorized::*;
pub use delegation::*;
pub use lockup::*;
pub use meta::*;
use pinocchio::account_info::{AccountInfo, Ref, RefMut},

pub use stake::*;
pub use stake_authorize::*;
pub use stake_clock::*;
pub use stake_flags::*;
pub use stake_history::*;
pub use stake_history_sysvar::*;
pub use stake_state_v2::*;
pub use utils::*;
pub use redelegate_state::*;


use crate::consts::{TAG_INITIALIZED, TAG_REWARDS_POOL, TAG_STAKE, TAG_UNINITIALIZED};

pub type Epoch = [u8; 8]; //u64
pub type UnixTimestamp = [u8; 8]; //i64;

pub fn get_stake_state(data: &[u8]) -> Result<StakeStateV2, ProgramError> {
    if data.len() != 200 {
        return Err(ProgramError::InvalidAccountData);
    }

    let tag = u32::from_le_bytes(
        data[0..4]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );

    let data_ptr = &data[4] as *const u8;

    unsafe {
        match tag {
            TAG_UNINITIALIZED => Ok(StakeStateV2::Uninitialized),
            TAG_INITIALIZED => {
                let meta = *(data_ptr as *const Meta);
                Ok(StakeStateV2::Initialized(meta))
            }
            TAG_STAKE => {
                let meta = *(data_ptr as *const Meta);
                let stake_ptr = data_ptr.add(core::mem::size_of::<Meta>());
                let stake = *(stake_ptr as *const Stake);
                let flags_ptr = stake_ptr.add(core::mem::size_of::<Stake>());
                let flags = *(flags_ptr as *const StakeFlags);
                Ok(StakeStateV2::Stake(meta, stake, flags))
            }
            TAG_REWARDS_POOL => Ok(StakeStateV2::RewardsPool),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}

pub fn try_get_stake_state_mut(
    stake_account_info: &AccountInfo,
) -> Result<RefMut<StakeStateV2>, ProgramError> {
    if stake_account_info.is_owned_by(&crate::ID) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    StakeStateV2::try_from_account_info_mut(stake_account_info)
}

pub fn set_stake_state(
    mut acc_stake_state_data: RefMut<[u8]>,
    new_state: StakeStateV2,
) -> ProgramResult {
    if acc_stake_state_data.len() != 200 {
        return Err(ProgramError::InvalidAccountData);
    }

    let (tag, body_bytes): (u32, [u8; 196]) = match new_state {
        StakeStateV2::Uninitialized => (TAG_UNINITIALIZED, [0u8; 196]),
        StakeStateV2::Initialized(meta) => {
            let mut body = [0u8; 196];
            let meta_bytes = unsafe {
                core::slice::from_raw_parts(
                    &meta as *const Meta as *const u8,
                    core::mem::size_of::<Meta>(),
                )
            };
            body[..meta_bytes.len()].copy_from_slice(meta_bytes);
            (TAG_INITIALIZED, body)
        }
        StakeStateV2::Stake(meta, stake, flags) => {
            let mut body = [0u8; 196];
            let meta_bytes = unsafe {
                core::slice::from_raw_parts(
                    &meta as *const Meta as *const u8,
                    core::mem::size_of::<Meta>(),
                )
            };
            let stake_bytes = unsafe {
                core::slice::from_raw_parts(
                    &stake as *const Stake as *const u8,
                    core::mem::size_of::<Stake>(),
                )
            };
            let flags_bytes = unsafe {
                core::slice::from_raw_parts(
                    &flags as *const StakeFlags as *const u8,
                    core::mem::size_of::<StakeFlags>(),
                )
            };

            let mut offset = 0;
            body[offset..offset + meta_bytes.len()].copy_from_slice(meta_bytes);
            offset += meta_bytes.len();
            body[offset..offset + stake_bytes.len()].copy_from_slice(stake_bytes);
            offset += stake_bytes.len();
            body[offset..offset + flags_bytes.len()].copy_from_slice(flags_bytes);

            (TAG_STAKE, body)
        }
        StakeStateV2::RewardsPool => (TAG_REWARDS_POOL, [0u8; 196]),
    };

    acc_stake_state_data[..4].copy_from_slice(&tag.to_le_bytes());
    acc_stake_state_data[4..].copy_from_slice(&body_bytes);
    Ok(())
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

