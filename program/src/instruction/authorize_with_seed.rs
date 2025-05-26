use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    ProgramResult,
};

use crate::state::{
    add_signer, clock_from_account_info, collect_signers_checked, do_authorize, StakeAuthorize,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AuthorizeWithSeedArgs<'a> {
    pub new_authorized_pubkey: Pubkey,
    pub stake_authorize: StakeAuthorize,
    pub authority_seed: &'a str,
    pub authority_owner: Pubkey,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AuthorizeCheckedWithSeedArgs<'a> {
    pub authority_owner: Pubkey,
    pub authority_seed_len: u32,
    // 4 bytes padding
    pub authority_seed: &'a str,
    pub stake_authorize: StakeAuthorize,
    // 7 bytes
}

// Borsh
// 10 (4bytes)
// abcdefghij (10 bytes)
// 111..32 (32 bytes)
// 1 (byte)

pub fn process_authorize_with_seed(
    accounts: &[AccountInfo],
    authorize_args: AuthorizeWithSeedArgs,
) -> ProgramResult {
    let [stake_account_info, stake_or_withdraw_authority_base_info, clock_info, remaining @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let clock = clock_from_account_info(clock_info)?;

    // other accounts
    let option_lockup_authority_info = remaining.first();

    let (mut signers, custodian, mut signers_count) =
        collect_signers_checked(None, option_lockup_authority_info)?;

    let seeds = &[
        stake_or_withdraw_authority_base_info.key().as_ref(),
        authorize_args.authority_seed.as_bytes(),
        authorize_args.authority_owner.as_ref(),
    ];
    let derived_key = pubkey::checked_create_program_address(seeds, &crate::id())?;

    if stake_or_withdraw_authority_base_info.is_signer() {
        add_signer(&mut signers, &mut signers_count, &derived_key)?;
    }

    do_authorize(
        stake_account_info,
        &signers,
        &authorize_args.new_authorized_pubkey,
        authorize_args.stake_authorize,
        custodian,
        &clock,
    )?;

    Ok(())
}
