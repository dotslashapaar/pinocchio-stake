use pinocchio::{
    account_info::{AccountInfo, Ref, RefMut},
    program_error::ProgramError,
};

use super::{Authorized, Delegation, Lockup, Meta, Stake, StakeFlags};

#[repr(C)]
#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum StakeStateV2 {
    Uninitialized,
    Initialized(Meta),
    Stake(Meta, Stake, StakeFlags),
    RewardsPool,
}

impl<'a> StakeStateV2 {
    /// The fixed number of bytes used to serialize each stake account
    pub const fn size_of() -> usize {
        200
    }

    #[inline]
    pub fn from_account_info(
        account_info: &AccountInfo,
    ) -> Result<Ref<StakeStateV2>, ProgramError> {
        if account_info.data_len() != Self::size_of() {
            return Err(ProgramError::InvalidAccountData);
        }

        let data = account_info.try_borrow_data()?;
        if !Self::is_aligned_to_4(&*data) || data[0] > 3 {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Ref::map(data, |data| unsafe { Self::from_bytes(data) }))
    }

    /// # Safety
    ///
    /// The caller must ensure that it is safe to borrow the account data – e.g., there are
    /// no mutable borrows of the account data.
    #[inline]
    pub unsafe fn from_account_info_unchecked(
        account_info: &AccountInfo,
    ) -> Result<&StakeStateV2, ProgramError> {
        if account_info.data_len() != Self::size_of() {
            return Err(ProgramError::InvalidAccountData);
        }
        let data = account_info.borrow_data_unchecked();
        if !Self::is_aligned_to_4(data) || data[0] > 3 {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self::from_bytes(data))
    }

    #[inline]
    pub fn try_from_account_info_mut(
        account_info: &AccountInfo,
    ) -> Result<RefMut<StakeStateV2>, ProgramError> {
        if account_info.data_len() != Self::size_of() {
            return Err(ProgramError::InvalidAccountData);
        }

        let data = account_info.try_borrow_mut_data()?;
        if !Self::is_aligned_to_4(&*data) || data[0] > 3 {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(RefMut::map(data, |data| unsafe {
            Self::from_bytes_mut(data)
        }))
    }

    /// # Safety
    ///
    /// The caller must ensure that it is safe to borrow the account data – e.g., there are
    /// no mutable borrows of the account data.
    #[inline]
    pub unsafe fn from_account_info_mut_unchecked(
        account_info: &AccountInfo,
    ) -> Result<&mut StakeStateV2, ProgramError> {
        if account_info.data_len() != Self::size_of() {
            return Err(ProgramError::InvalidAccountData);
        }
        let data = account_info.borrow_mut_data_unchecked();
        if !Self::is_aligned_to_4(data) || data[0] > 3 {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self::from_bytes_mut(data))
    }
    /// # Safety
    ///
    /// The caller must ensure that `bytes` contains a valid representation of `StakeStateV2`.
    #[inline(always)]
    pub unsafe fn from_bytes(bytes: &[u8]) -> &Self {
        &*(bytes.as_ptr() as *const Self)
    }

    /// # Safety
    ///
    /// The caller must ensure that `bytes` contains a valid representation of `StakeStateV2`.
    #[inline(always)]
    pub unsafe fn from_bytes_mut(bytes: &mut [u8]) -> &mut Self {
        &mut *(bytes.as_mut_ptr() as *mut Self)
    }

    fn is_aligned_to_4(data: &[u8]) -> bool {
        let ptr = data.as_ptr() as usize;
        ptr % 4 == 0
    }

    pub fn stake(&self) -> Option<Stake> {
        match self {
            Self::Stake(_meta, stake, _stake_flags) => Some(*stake),
            Self::Uninitialized | Self::Initialized(_) | Self::RewardsPool => None,
        }
    }

    pub fn stake_ref(&self) -> Option<&Stake> {
        match self {
            Self::Stake(_meta, stake, _stake_flags) => Some(stake),
            Self::Uninitialized | Self::Initialized(_) | Self::RewardsPool => None,
        }
    }

    pub fn delegation(&self) -> Option<Delegation> {
        match self {
            Self::Stake(_meta, stake, _stake_flags) => Some(stake.delegation),
            Self::Uninitialized | Self::Initialized(_) | Self::RewardsPool => None,
        }
    }

    pub fn delegation_ref(&self) -> Option<&Delegation> {
        match self {
            StakeStateV2::Stake(_meta, stake, _stake_flags) => Some(&stake.delegation),
            Self::Uninitialized | Self::Initialized(_) | Self::RewardsPool => None,
        }
    }

    pub fn authorized(&self) -> Option<Authorized> {
        match self {
            Self::Stake(meta, _stake, _stake_flags) => Some(meta.authorized),
            Self::Initialized(meta) => Some(meta.authorized),
            Self::Uninitialized | Self::RewardsPool => None,
        }
    }

    pub fn lockup(&self) -> Option<Lockup> {
        self.meta().map(|meta| meta.lockup)
    }

    pub fn meta(&self) -> Option<Meta> {
        match self {
            Self::Stake(meta, _stake, _stake_flags) => Some(*meta),
            Self::Initialized(meta) => Some(*meta),
            Self::Uninitialized | Self::RewardsPool => None,
        }
    }
}
#[cfg(test)]
mod test {
    use super::StakeStateV2;

    #[test]
    fn test_from_initialized() {
        // StakeStateV2 Initialized(Meta { rent_exempt_reserve: 2282880, authorized: Authorized { staker: 531ngDyMQ95Ws12uWwf9k8bcBqtTWQ4enhNr9zKFZTHV, withdrawer: 531ngDyMQ95Ws12uWwf9k8bcBqtTWQ4enhNr9zKFZTHV }, lockup: Lockup { unix_timestamp: 0, epoch: 1, custodian: FAp2uc71WiitTgf8C4EzT9CNboKs9j8UnNAA2zJhpmNo } })
        let data: [u8; 200] = [
            1, 0, 0, 0, 128, 213, 34, 0, 0, 0, 0, 0, 59, 242, 204, 190, 54, 61, 5, 33, 184, 22,
            185, 9, 8, 116, 164, 194, 234, 165, 126, 13, 237, 190, 6, 236, 191, 198, 111, 157, 70,
            124, 157, 196, 59, 242, 204, 190, 54, 61, 5, 33, 184, 22, 185, 9, 8, 116, 164, 194,
            234, 165, 126, 13, 237, 190, 6, 236, 191, 198, 111, 157, 70, 124, 157, 196, 0, 0, 0, 0,
            0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 210, 135, 6, 69, 103, 142, 166, 59, 132, 215, 180,
            188, 12, 10, 104, 133, 78, 242, 108, 76, 169, 33, 196, 149, 254, 142, 141, 219, 44, 39,
            252, 88, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let val = unsafe { &*(data.as_ptr() as *const StakeStateV2) };

        println!("{:?}", val);
    }

    #[test]
    fn test_from_stake() {
        // StakeStateV2 Stake(Meta { rent_exempt_reserve: 0, authorized: Authorized { staker: CJbnEm6uEhUQHyFt8bsYfDobbx6b39r47X4To5S89qRP, withdrawer: CJbnEm6uEhUQHyFt8bsYfDobbx6b39r47X4To5S89qRP }, lockup: Lockup { unix_timestamp: 0, epoch: 0, custodian: 11111111111111111111111111111111 } }, Stake { delegation: Delegation { voter_pubkey: DBF6UmjTW3vY5y58J5f3ePW9sMPgJ2wWJAygpFPsJxT4, stake: 1, activation_epoch: 1, deactivation_epoch: 18446744073709551615, warmup_cooldown_rate: 0.25 }, credits_observed: 969 }, StakeFlags { bits: 0 })
        let data: [u8; 200] = [
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 167, 242, 193, 121, 156, 42, 145, 92, 134, 135, 64,
            238, 153, 60, 83, 202, 158, 70, 169, 101, 171, 142, 71, 92, 44, 123, 106, 167, 183, 80,
            65, 150, 167, 242, 193, 121, 156, 42, 145, 92, 134, 135, 64, 238, 153, 60, 83, 202,
            158, 70, 169, 101, 171, 142, 71, 92, 44, 123, 106, 167, 183, 80, 65, 150, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 180, 235, 252, 228, 206, 204, 148, 35, 80,
            199, 23, 103, 170, 175, 11, 213, 246, 90, 116, 128, 217, 88, 50, 227, 163, 43, 95, 192,
            68, 203, 54, 43, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255,
            255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 208, 63, 201, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let val = unsafe { &*(data.as_ptr() as *const StakeStateV2) };

        println!("{:?}", val);
    }
}
