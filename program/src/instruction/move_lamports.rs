use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};

use crate::{
    helpers::MergeKind,
    state::{
        collect_signers_checked, get_stake_state, next_account_info, relocate_lamports,
        to_program_error, StakeAuthorize, StakeHistorySysvar,
    },
};

// #[cfg(feature = "bincode")]
// pub fn move_lamports<'a>(
//     source_stake_pubkey: &'a Pubkey,
//     destination_stake_pubkey: &'a Pubkey,
//     authorized_pubkey: &'a Pubkey,
//     lamports: u64,
// ) -> Instruction<'a, 'a, 'a, 'a> {
//     let account_metas = vec![
//         AccountMeta::new(source_stake_pubkey, false, false),
//         AccountMeta::new(destination_stake_pubkey, false, false),
//         AccountMeta::readonly(authorized_pubkey),
//     ];

//     Instruction::new_with_bincode(ID, &StakeInstruction::MoveLamports(lamports), account_metas)
// }

fn move_stake_or_lamports_shared_checks(
    source_stake_account_info: &AccountInfo,
    lamports: u64,
    destination_stake_account_info: &AccountInfo,
    stake_authority_info: &AccountInfo,
) -> Result<(MergeKind, MergeKind), ProgramError> {
    // authority must sign
    let (signers, _, _) = collect_signers_checked(Some(stake_authority_info), None)?;

    // confirm not the same account
    if *source_stake_account_info.key() == *destination_stake_account_info.key() {
        return Err(ProgramError::InvalidInstructionData);
    }

    // source and destination must be writable
    // runtime guards against unowned writes, but MoveStake and MoveLamports are defined by SIMD
    // we check explicitly to avoid any possibility of a successful no-op that never attempts to write
    if !source_stake_account_info.is_writable() || !destination_stake_account_info.is_writable() {
        return Err(ProgramError::InvalidInstructionData);
    }

    // must move something
    if lamports == 0 {
        return Err(ProgramError::InvalidArgument);
    }

    let clock = Clock::get()?;
    let stake_history = StakeHistorySysvar(clock.epoch);

    // get_if_mergeable ensures accounts are not partly activated or in any form of deactivating
    // we still need to exclude activating state ourselves
    let source_merge_kind = MergeKind::get_if_mergeable(
        &*get_stake_state(source_stake_account_info)?,
        source_stake_account_info.lamports(),
        &clock,
        &stake_history,
    )?;

    // Authorized staker is allowed to move stake
    source_merge_kind
        .meta()
        .authorized
        .check(&signers, StakeAuthorize::Staker)
        .map_err(to_program_error)?;

    // same transient assurance as with source
    let destination_merge_kind = MergeKind::get_if_mergeable(
        &*get_stake_state(destination_stake_account_info)?,
        destination_stake_account_info.lamports(),
        &clock,
        &stake_history,
    )?;

    // ensure all authorities match and lockups match if lockup is in force
    MergeKind::metas_can_merge(
        source_merge_kind.meta(),
        destination_merge_kind.meta(),
        &clock,
    )?;

    Ok((source_merge_kind, destination_merge_kind))
}

fn process_move_lamports(accounts: &[AccountInfo], lamports: u64) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // native asserts: 3 accounts
    let source_stake_account_info = next_account_info(account_info_iter)?;
    let destination_stake_account_info = next_account_info(account_info_iter)?;
    let stake_authority_info = next_account_info(account_info_iter)?;

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
