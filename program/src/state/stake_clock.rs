use pinocchio::{
    account_info::{AccountInfo, Ref},
    program_error::ProgramError,
};

use crate::consts::SYSVAR;

pub type Slot = [u8; 8];
pub type Epoch = [u8; 8];
pub type UnixTimestamp = [u8; 8];

/// A representation of network time.
///
/// All members of `Clock` start from 0 upon network boot.
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Clock {
    pub slot: Slot,
    pub epoch_start_timestamp: UnixTimestamp,
    pub epoch: Epoch,
    pub leader_schedule_epoch: Epoch,
    pub unix_timestamp: UnixTimestamp,
}

impl Clock {
    //Clock doesn't have a from_account_info, so we implemt it, inspired from TokenAccount Pinocchio impl
    pub fn from_account_info(account_info: &AccountInfo) -> Result<Ref<Clock>, ProgramError> {
        if account_info.data_len() != core::mem::size_of::<Clock>() {
            return Err(ProgramError::InvalidAccountData);
        }

        if !account_info.is_owned_by(&SYSVAR) {
            return Err(ProgramError::InvalidAccountData);
        }

        let data = account_info.try_borrow_data()?;

        Ok(Ref::map(data, |data| unsafe { Self::from_bytes(data) }))
    }

    /// # Safety
    ///
    /// The caller must ensure that `bytes` contains a valid representation of `Clock`.
    #[inline(always)]
    pub unsafe fn from_bytes(bytes: &[u8]) -> &Self {
        &*(bytes.as_ptr() as *const Self)
    }
}
