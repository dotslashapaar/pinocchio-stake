use pinocchio::{program_error::ProgramError, pubkey::Pubkey, sysvars::clock::Clock};

use crate::error::StakeError;

use super::{Lockup, StakeAuthorize};

#[repr(C)]
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub struct Authorized {
    pub staker: Pubkey,
    pub withdrawer: Pubkey,
}

impl Authorized {
    pub fn auto(authorized: &Pubkey) -> Self {
        Self {
            staker: *authorized,
            withdrawer: *authorized,
        }
    }

    pub fn check(
        &self,
        signers: &[Pubkey],
        stake_authorize: StakeAuthorize,
    ) -> Result<(), ProgramError> {
        let authorized_signer = match stake_authorize {
            StakeAuthorize::Staker => &self.staker,
            StakeAuthorize::Withdrawer => &self.withdrawer,
        };
        if signers.iter().any(|p| p == authorized_signer) {
            Ok(())
        } else {
            Err(ProgramError::MissingRequiredSignature)
        }
    }

    pub fn authorize(
        &mut self,
        signers: &[Pubkey],
        new_authorized: &Pubkey,
        stake_authorize: StakeAuthorize,
        lockup_custodian_args: Option<(&Lockup, &Clock, Option<&Pubkey>)>,
    ) -> Result<(), ProgramError> {
        match stake_authorize {
            StakeAuthorize::Staker => {
                // Allow either the staker or the withdrawer to change the staker key
                if !signers.contains(&self.staker) && !signers.contains(&self.withdrawer) {
                    return Err(ProgramError::MissingRequiredSignature);
                }
                self.staker = *new_authorized
            }
            StakeAuthorize::Withdrawer => {
                if let Some((lockup, clock, custodian)) = lockup_custodian_args {
                    if lockup.is_in_force(clock, None) {
                        match custodian {
                            None => {
                                return Err(StakeError::CustodianMissing.into());
                            }
                            Some(custodian) => {
                                if !signers.contains(custodian) {
                                    return Err(StakeError::CustodianSignatureMissing.into());
                                }

                                if lockup.is_in_force(clock, Some(custodian)) {
                                    return Err(StakeError::LockupInForce.into());
                                }
                            }
                        }
                    }
                }
                self.check(signers, stake_authorize)?;
                self.withdrawer = *new_authorized
            }
        }
        Ok(())
    }
}
