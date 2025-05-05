use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};

use crate::{
    helpers::MergeKind,
    state::{move_stake_or_lamports_shared_checks, relocate_lamports},
};

pub fn process_move_lamports(accounts: &[AccountInfo], lamports: u64) -> ProgramResult {
    let [source_stake_account_info, destination_stake_account_info, stake_authority_info, _remaining @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let (source_merge_kind, _) = move_stake_or_lamports_shared_checks(
        source_stake_account_info,
        lamports,
        destination_stake_account_info,
        stake_authority_info,
    )?;

    let source_free_lamports = match source_merge_kind {
        MergeKind::FullyActive(source_meta, source_stake) => source_stake_account_info
            .lamports()
            .saturating_sub(u64::from_le_bytes(source_stake.delegation.stake))
            .saturating_sub(u64::from_le_bytes(source_meta.rent_exempt_reserve)),
        MergeKind::Inactive(source_meta, source_lamports, _) => {
            source_lamports.saturating_sub(u64::from_le_bytes(source_meta.rent_exempt_reserve))
        }
        _ => return Err(ProgramError::InvalidAccountData),
    };

    if lamports > source_free_lamports {
        return Err(ProgramError::InvalidArgument);
    }

    relocate_lamports(
        source_stake_account_info,
        destination_stake_account_info,
        lamports,
    )?;

    Ok(())
}
