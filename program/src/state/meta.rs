use pinocchio::sysvars::clock::Clock;

use crate::{error::InstructionError, instruction::LockupArgs};

use super::{Authorized, Lockup};

#[repr(C)]
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub struct Meta {
    rent_exempt_reserve: [u8; 8], // u64
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
        has_custodian_signer: bool,
        has_withdrawer_signer: bool,
        clock: &Clock,
    ) -> Result<(), InstructionError> {
        // post-stake_program_v4 behavior:
        // * custodian can update the lockup while in force
        // * withdraw authority can set a new lockup
        if self.lockup.is_in_force(clock, None) {
            if !has_custodian_signer {
                return Err(InstructionError::MissingRequiredSignature);
            }
        } else if !has_withdrawer_signer {
            return Err(InstructionError::MissingRequiredSignature);
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
}
