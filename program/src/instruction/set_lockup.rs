use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};

use crate::{
    error::to_program_error,
    state::{get_stake_state, try_get_stake_state_mut, Epoch, StakeStateV2, UnixTimestamp},
};

#[cfg(not(test))]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LockupArgs {
    pub unix_timestamp: Option<UnixTimestamp>,
    pub epoch: Option<Epoch>,
    pub custodian: Option<Pubkey>,
}

#[cfg(test)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct LockupArgs {
    pub unix_timestamp: Option<UnixTimestamp>,
    pub epoch: Option<Epoch>,
    pub custodian: Option<Pubkey>,
}

impl LockupArgs {
    pub fn from_data(data: &[u8]) -> Result<Self, ProgramError> {
        match data.len() {
            // all none: 1 + 1 + 1
            3 => {
                if (data[0] == 1) || (data[1] == 1) || (data[2] == 1) {
                    return Err(ProgramError::InvalidInstructionData);
                }
                Ok(LockupArgs {
                    unix_timestamp: None,
                    epoch: None,
                    custodian: None,
                })
            }
            // (unix_timestamp - some, other - none) or (epoch - some, other none): 9 + 1 + 1
            11 => {
                if !(((data[0] == 1) && (data[9] == 0) && (data[10] == 0))
                    || ((data[0] == 0) && (data[1] == 1) && (data[10] == 0)))
                {
                    return Err(ProgramError::InvalidInstructionData);
                }
                if data[0] == 1 {
                    Ok(LockupArgs {
                        unix_timestamp: Some(unsafe {
                            *(data[1..=8].as_ptr() as *const UnixTimestamp)
                        }),
                        epoch: None,
                        custodian: None,
                    })
                } else {
                    Ok(LockupArgs {
                        unix_timestamp: None,
                        epoch: Some(unsafe { *(data[2..=9].as_ptr() as *const Epoch) }),
                        custodian: None,
                    })
                }
            }
            // (unix_timestamp and epoch - some, custodian - none): 9 + 9 + 1
            19 => {
                if !((data[0] == 1) && (data[9] == 1) && (data[18] == 0)) {
                    return Err(ProgramError::InvalidInstructionData);
                }
                Ok(LockupArgs {
                    unix_timestamp: Some(unsafe {
                        *(data[1..=8].as_ptr() as *const UnixTimestamp)
                    }),
                    epoch: Some(unsafe { *(data[10..=17].as_ptr() as *const Epoch) }),
                    custodian: None,
                })
            }
            // (custodian - some, other - none): 1 + 1 + 33
            35 => {
                if !((data[0] == 0) && (data[1] == 0) && (data[2] == 1)) {
                    return Err(ProgramError::InvalidInstructionData);
                }
                Ok(LockupArgs {
                    unix_timestamp: None,
                    epoch: None,
                    custodian: Some(unsafe { *(data[3..=34].as_ptr() as *const Pubkey) }),
                })
            }
            // (custodian - some, either unix_timestamp or epoch - none): 9 + 1 + 33
            43 => {
                if !(((data[0] == 0) && (data[1] == 1) && (data[10] == 1))
                    || ((data[0] == 1) && (data[9] == 0) && (data[10] == 1)))
                {
                    return Err(ProgramError::InvalidInstructionData);
                }
                if data[0] == 1 {
                    Ok(LockupArgs {
                        unix_timestamp: Some(unsafe {
                            *(data[1..=8].as_ptr() as *const UnixTimestamp)
                        }),
                        epoch: None,
                        custodian: Some(unsafe { *(data[11..=42].as_ptr() as *const Pubkey) }),
                    })
                } else {
                    Ok(LockupArgs {
                        unix_timestamp: None,
                        epoch: Some(unsafe { *(data[2..=9].as_ptr() as *const Epoch) }),
                        custodian: Some(unsafe { *(data[11..=42].as_ptr() as *const Pubkey) }),
                    })
                }
            }
            // all some: 9 + 9 + 33
            51 => {
                if !((data[0] == 1) && (data[9] == 1) && (data[18] == 1)) {
                    return Err(ProgramError::InvalidInstructionData);
                }
                Ok(unsafe { *(data.as_ptr() as *const Self) })
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        }
    }
}

pub fn process_set_lockup(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let lockup_args = LockupArgs::from_data(data)?;

    let [stake_account_info, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let stake_account: pinocchio::account_info::Ref<'_, StakeStateV2> =
        get_stake_state(stake_account_info)?;

    // check signers
    let mut has_custodian_signer = false;
    let mut has_withdrawer_signer = false;
    match *stake_account {
        StakeStateV2::Initialized(ref meta) | StakeStateV2::Stake(ref meta, _, _) => {
            for account in accounts {
                if account.is_signer() {
                    if meta.lockup.custodian == *account.key() {
                        has_custodian_signer = true;
                    }
                    if meta.authorized.withdrawer == *account.key() {
                        has_withdrawer_signer = true;
                    }
                }
            }
        }
        _ => {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let clock = Clock::get()?;

    do_set_lookup(
        stake_account_info,
        &lockup_args,
        has_custodian_signer,
        has_withdrawer_signer,
        &clock,
    )?;

    Ok(())
}

fn do_set_lookup(
    stake_account_info: &AccountInfo,
    lockup: &LockupArgs,
    has_custodian_signer: bool,
    has_withdrawer_signer: bool,
    clock: &Clock,
) -> ProgramResult {
    let mut stake_account: pinocchio::account_info::RefMut<'_, StakeStateV2> =
        try_get_stake_state_mut(stake_account_info)?;
    match *stake_account {
        StakeStateV2::Initialized(ref mut meta) => meta
            .set_lockup(lockup, has_custodian_signer, has_withdrawer_signer, clock)
            .map_err(to_program_error),
        StakeStateV2::Stake(ref mut meta, _stake, _stake_flags) => meta
            .set_lockup(lockup, has_custodian_signer, has_withdrawer_signer, clock)
            .map_err(to_program_error),
        _ => Err(ProgramError::InvalidAccountData),
    }
}

#[cfg(test)]
mod test {
    use super::LockupArgs;
    use bincode::serialize;

    #[test]
    fn test_instruction_data() {
        let args_arr = [
            LockupArgs {
                unix_timestamp: None,
                epoch: None,
                custodian: None,
            },
            LockupArgs {
                unix_timestamp: Some(3609733389592650838i64.to_le_bytes()),
                epoch: None,
                custodian: None,
            },
            LockupArgs {
                unix_timestamp: None,
                epoch: Some(9464321479845648u64.to_le_bytes()),
                custodian: None,
            },
            LockupArgs {
                unix_timestamp: None,
                epoch: None,
                custodian: Some([
                    13, 54, 98, 123, 59, 67, 165, 78, 03, 12, 23, 45, 67, 89, 01, 02, 03, 04, 05,
                    06, 07, 08, 09, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                ]),
            },
            LockupArgs {
                unix_timestamp: Some(3609733389592650838i64.to_le_bytes()),
                epoch: Some(9464321479845648u64.to_le_bytes()),
                custodian: None,
            },
            LockupArgs {
                unix_timestamp: Some(3609733389592650838i64.to_le_bytes()),
                epoch: None,
                custodian: Some([
                    13, 54, 98, 123, 59, 67, 165, 78, 03, 12, 23, 45, 67, 89, 01, 02, 03, 04, 05,
                    06, 07, 08, 09, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                ]),
            },
            LockupArgs {
                unix_timestamp: None,
                epoch: Some(9464321479845648u64.to_le_bytes()),
                custodian: Some([
                    13, 54, 98, 123, 59, 67, 165, 78, 03, 12, 23, 45, 67, 89, 01, 02, 03, 04, 05,
                    06, 07, 08, 09, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                ]),
            },
            LockupArgs {
                unix_timestamp: Some(3609733389592650838i64.to_le_bytes()),
                epoch: Some(9464321479845648u64.to_le_bytes()),
                custodian: Some([
                    13, 54, 98, 123, 59, 67, 165, 78, 03, 12, 23, 45, 67, 89, 01, 02, 03, 04, 05,
                    06, 07, 08, 09, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                ]),
            },
        ];

        for args in args_arr {
            let data = serialize(&args).unwrap();

            let args_new = LockupArgs::from_data(data.as_ref()).unwrap();
            assert_eq!(args, args_new);
        }
    }
}
