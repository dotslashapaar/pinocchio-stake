use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

use crate::state::{clock_from_account_info, collect_signers, do_authorize, StakeAuthorize};

pub fn process_authorize_checked(
    accounts: &[AccountInfo],
    authority_type: StakeAuthorize,
) -> ProgramResult {
    let mut signers = [Pubkey::default(); 32];
    let _signers_len = collect_signers(accounts, &mut signers)?;

    let [stake_account_info, clock_info, _old_stake_or_withdraw_authority_info, new_stake_or_withdraw_authority_info, rest @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // other accounts
    let option_lockup_authority_info = if !rest.is_empty() {
        Some(&rest[0])
    } else {
        None
    };

    let clock = *clock_from_account_info(clock_info)?;

    if !new_stake_or_withdraw_authority_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let custodian = option_lockup_authority_info
        .filter(|a| a.is_signer())
        .map(|a| a.key());

    // `get_stake_state()` is called unconditionally, which checks owner
    do_authorize(
        stake_account_info,
        &signers,
        new_stake_or_withdraw_authority_info.key(),
        authority_type,
        custodian,
        clock,
    )?;

    Ok(())
}
