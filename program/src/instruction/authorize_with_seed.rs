use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;

use crate::state::{
    collect_signers_checked, get_stake_state, next_account_info, set_stake_state, to_program_error,
    StakeAuthorize, StakeStateV2,
};

#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Deserialize, serde_derive::Serialize)
)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AuthorizeWithSeedArgs {
    pub new_authorized_pubkey: Pubkey,
    pub stake_authorize: StakeAuthorize,
    pub authority_seed: Pubkey,
    pub authority_owner: Pubkey,
}

fn process_authorize_with_seed(
    accounts: &[AccountInfo],
    authorize_args: AuthorizeWithSeedArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // native asserts: 3 accounts (1 sysvar)
    let stake_account_info = next_account_info(account_info_iter)?;
    let stake_or_withdraw_authority_base_info = next_account_info(account_info_iter)?;
    let clock_info = next_account_info(account_info_iter)?;

    // other accounts
    let option_lockup_authority_info = next_account_info(account_info_iter).ok();

    // let clock = &Clock::from_account_info(clock_info)?; - temp
    let clock = &Clock::get()?;

    let (mut signers, custodian) = collect_signers_checked(None, option_lockup_authority_info)?;

    if stake_or_withdraw_authority_base_info.is_signer() {
        signers.insert(
            CreateAccount {
                from: stake_or_withdraw_authority_base_info,
                to: stake_account_info,
                lamports: 0,
                owner: &authorize_args.authority_owner,
                space: 0,
            }
            .invoke_signed(&[&[signers]])?,
        );

        // signers.insert(Pubkey::create_with_seed(
        //     stake_or_withdraw_authority_base_info.key(),
        //     &authorize_args.authority_seed,
        //     &authorize_args.authority_owner,
        // )?);
    }

    // `get_stake_state()` is called unconditionally, which checks owner
    do_authorize(
        stake_account_info,
        &signers,
        &authorize_args.new_authorized_pubkey,
        authorize_args.stake_authorize,
        custodian,
        clock,
    )?;

    Ok(())
}

fn do_authorize(
    stake_account_info: &AccountInfo,
    signers: &HashSet<Pubkey>,
    new_authority: &Pubkey,
    authority_type: StakeAuthorize,
    custodian: Option<&Pubkey>,
    clock: &Clock,
) -> ProgramResult {
    match *get_stake_state(stake_account_info)? {
        StakeStateV2::Initialized(mut meta) => {
            meta.authorized
                .authorize(
                    signers,
                    new_authority,
                    authority_type,
                    Some((&meta.lockup, clock, custodian)),
                )
                .map_err(to_program_error)?;

            set_stake_state(stake_account_info, &StakeStateV2::Initialized(meta))
        }
        StakeStateV2::Stake(mut meta, stake, stake_flags) => {
            meta.authorized
                .authorize(
                    signers,
                    new_authority,
                    authority_type,
                    Some((&meta.lockup, clock, custodian)),
                )
                .map_err(to_program_error)?;

            set_stake_state(
                stake_account_info,
                &StakeStateV2::Stake(meta, stake, stake_flags),
            )
        }
        _ => Err(ProgramError::InvalidAccountData),
    }
}
