use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};

use pinocchio::instruction::{Seed, Signer};

use pinocchio_token::{instructions::TransferChecked, state::{TokenAccount, Mint}};

use crate::state::load_acc_mut_unchecked;

use crate::{
    state::{
        utils::{load_ix_data, DataLen},
        RedelegateState,
    },
};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StartRedelegationIxData {
    pub new_validator: Pubkey,
    pub stake_amount: [u8; 8],
    pub bump: u8,
}

impl DataLen for StartRedelegationIxData {
    const LEN: usize = core::mem::size_of::<StartRedelegationIxData>();
}

pub fn process_start_redelegation(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [owner_acc, state_acc, new_validator_acc] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !owner_acc.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let ix_data = unsafe { load_ix_data::<StartRedelegationIxData>(data)? };

    let redelegate_state = unsafe {
        load_acc_mut_unchecked::<RedelegateState>(state_acc.borrow_mut_data_unchecked())
    }?;

    if !redelegate_state.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }

    if redelegate_state.owner != *owner_acc.key() {
        return Err(ProgramError::IllegalOwner);
    }

    if ix_data.new_validator != *new_validator_acc.key() {
        return Err(ProgramError::InvalidArgument);
    }

    redelegate_state.start_redelegation(&ix_data)
}

pub fn process_complete_redelegation(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [owner_acc, owner_ata, mint_to_stake, vault, state_acc, current_validator_acc, new_validator_acc] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let vault_acc = TokenAccount::from_account_info(vault)?;

    assert_eq!(vault_acc.owner(), state_acc.key());

    let owner_ata_acc = TokenAccount::from_account_info(owner_ata)?;
    assert_eq!(owner_ata_acc.owner(), owner_acc.key());

    let mint_state = Mint::from_account_info(mint_to_stake)?;

    if !owner_acc.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let redelegate_state = unsafe {
        load_acc_mut_unchecked::<RedelegateState>(state_acc.borrow_mut_data_unchecked())
    }?;

    if !redelegate_state.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }

    if redelegate_state.owner != *owner_acc.key() {
        return Err(ProgramError::IllegalOwner);
    }

    if redelegate_state.current_validator != *current_validator_acc.key() {
        return Err(ProgramError::InvalidArgument);
    }

    if redelegate_state.new_validator != *new_validator_acc.key() {
        return Err(ProgramError::InvalidArgument);
    }

    let ix_data = unsafe { load_ix_data::<StartRedelegationIxData>(data)? };

    redelegate_state.complete_redelegation();

    let stake_amount = u64::from_le_bytes(ix_data.stake_amount);
    if stake_amount > vault_acc.amount() {
        (TransferChecked{
            from: owner_ata,
            to: vault,
            mint: mint_to_stake,
            authority: owner_acc,
            amount: stake_amount - vault_acc.amount(),
            decimals: mint_state.decimals(),
        }).invoke()
    }
    else {
        let bump = &[ix_data.bump];
        let seeds = &[
            Seed::from(RedelegateState::SEED.as_bytes()),
            Seed::from(owner_acc.key().as_ref()),
            Seed::from(state_acc.key().as_ref()),
            Seed::from(bump),
        ];
        let signer = Signer::from(seeds);

        (TransferChecked{
            from: vault,
            to: owner_ata,
            mint: mint_to_stake,
            authority: state_acc,
            amount: vault_acc.amount() - stake_amount,
            decimals: mint_state.decimals(),
        }).invoke_signed(&[signer])
    }

}
