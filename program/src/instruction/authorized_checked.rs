use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

use crate::state::{collect_signers, do_authorize, next_account_info, Clock, StakeAuthorize};

pub fn process_authorize_checked(
    accounts: &[AccountInfo],
    authority_type: StakeAuthorize,
) -> ProgramResult {
    let mut signers = [Pubkey::default(); 32];
    let _signers_len = collect_signers(accounts, &mut signers)?;
    let account_info_iter = &mut accounts.iter();

    // native asserts: 4 accounts (1 sysvar)
    let stake_account_info = next_account_info(account_info_iter)?;
    let clock_info = next_account_info(account_info_iter)?; //Our own Struct
    let _old_stake_or_withdraw_authority_info = next_account_info(account_info_iter)?;
    let new_stake_or_withdraw_authority_info = next_account_info(account_info_iter)?;

    // other accounts
    let option_lockup_authority_info = next_account_info(account_info_iter).ok();

    let clock = *Clock::from_account_info(clock_info)?;

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
