use pinocchio::{program_error::ProgramError, pubkey::Pubkey, sysvars::clock::Clock};

use super::{Authorized, Epoch, Lockup};

pub type UnixTimestamp = [u8; 8];

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub struct LockupArgs {
    pub unix_timestamp: Option<UnixTimestamp>,
    pub epoch: Option<Epoch>,
    pub custodian: Option<Pubkey>,
}

#[repr(C)]
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub struct Meta {
    pub rent_exempt_reserve: [u8; 8], // u64
    pub authorized: Authorized,
    pub lockup: Lockup,
}

impl Meta {
    #[inline(always)]
    pub fn set_rent_exempt_reserve(&mut self, rent_exempt_reserve: u64) {
        self.rent_exempt_reserve = rent_exempt_reserve.to_le_bytes();
    }

    #[inline(always)]
    pub fn rent_exempt_reserve(&self) -> u64 {
        u64::from_le_bytes(self.rent_exempt_reserve)
    }

    pub fn set_lockup(
        &mut self,
        lockup: &LockupArgs,
        signers: &[Pubkey],
        clock: &Clock,
    ) -> Result<(), ProgramError> {
        // post-stake_program_v4 behavior:
        // * custodian can update the lockup while in force
        // * withdraw authority can set a new lockup
        if self.lockup.is_in_force(clock, None) {
            if !signers.contains(&self.lockup.custodian) {
                return Err(ProgramError::MissingRequiredSignature);
            }
        } else if !signers.contains(&self.authorized.withdrawer) {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if let Some(unix_timestamp) = lockup.unix_timestamp {
            self.lockup.unix_timestamp = unix_timestamp;
        }
        if let Some(epoch) = lockup.epoch {
            self.lockup.epoch = epoch;
        }
        if let Some(custodian) = lockup.custodian {
            self.lockup.custodian = custodian;
        }
        Ok(())
    }

    pub fn auto(authorized: &Pubkey) -> Self {
        Self {
            authorized: Authorized::auto(authorized),
            ..Meta::default()
        }
    }
}
