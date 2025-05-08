use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};
use crate::state::{
    bytes_to_u64,
    clock_from_account_info,
    collect_signers,
    get_stake_state,
    get_vote_state,
    new_stake,
    next_account_info,
    redelegate_stake,
    set_stake_state,
    to_program_error,
    validate_delegated_amount,
    StakeFlags,
    StakeHistorySysvar,
    StakeStateV2,
    ValidatedDelegatedInfo,
};

pub fn process_delegate(accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    let mut signers = [Pubkey::default(); 32];
    let _signers_len = collect_signers(accounts, &mut signers)?;

    // native accounts -- asserted
    let accounts_info_iter = &mut accounts.iter();
    let stake_account_info = next_account_info(accounts_info_iter)?;
    let vote_account_info = next_account_info(accounts_info_iter)?;
    let clock_info = next_account_info(accounts_info_iter)?;
    let _stake_history_info = next_account_info(accounts_info_iter)?;
    let _stake_config_info = next_account_info(accounts_info_iter)?;

    // for future refactors, after the bpf switchover we may assert them as well.
    // other account info
    // let _stake_authority_info = next_account_info(accounts_info_iter)?;

    let clock = clock_from_account_info(clock_info)?;
    let stake_history = &StakeHistorySysvar(bytes_to_u64(clock.epoch.to_le_bytes()));
    let vote_state = get_vote_state(vote_account_info)?;

    match *get_stake_state(stake_account_info)? {
        crate::state::StakeStateV2::Initialized(meta) => {
            meta.authorized
                .check(&signers, crate::state::StakeAuthorize::Staker)
                .map_err(to_program_error)?;
            let ValidatedDelegatedInfo { stake_amount } = validate_delegated_amount(
                stake_account_info,
                &meta
            )?;
            let stake = new_stake(
                stake_amount,
                vote_account_info.key(),
                &vote_state,
                clock.epoch.to_le_bytes()
            );
            set_stake_state(
                stake_account_info,
                &StakeStateV2::Stake(meta, stake, StakeFlags::empty())
            )?;
        }
        crate::state::StakeStateV2::Stake(meta, mut stake, flags) => {
            meta.authorized
                .check(&signers, crate::state::StakeAuthorize::Staker)
                .map_err(to_program_error)?;
            let ValidatedDelegatedInfo { stake_amount } = validate_delegated_amount(
                stake_account_info,
                &meta
            )?;

            redelegate_stake(
                &mut stake,
                stake_amount,
                vote_account_info.key(),
                &vote_state,
                clock.epoch.to_le_bytes(),
                stake_history
            )?;
            set_stake_state(stake_account_info, &StakeStateV2::Stake(meta, stake, flags))?;
        }
        _ => {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    Ok(())
}
