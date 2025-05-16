use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    sysvars::clock::Clock,
    ProgramResult,
};

use crate::state::{add_signer, collect_signers_checked, do_authorize, StakeAuthorize};

#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Deserialize, serde_derive::Serialize)
)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AuthorizeWithSeedArgs<'a> {
    pub new_authorized_pubkey: Pubkey,
    pub stake_authorize: StakeAuthorize,
    pub authority_seed: &'a str,
    pub authority_owner: Pubkey,
}

fn process_authorize_with_seed(
    accounts: &[AccountInfo],
    authorize_args: AuthorizeWithSeedArgs,
) -> ProgramResult {
    let [stake_account_info, stake_or_withdraw_authority_base_info, clock_info, remaining @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let clock = Clock::from_account_info(clock_info)?;

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
        add_signer(&mut signers, &mut signers_count, &derived_key);
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
