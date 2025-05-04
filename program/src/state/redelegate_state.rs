use super::utils::{load_acc_mut_unchecked, DataLen, Initialized};
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    ProgramResult,
};

use pinocchio::sysvars::Sysvar;
use pinocchio::sysvars::clock::Clock;

use crate::{
    instruction::StartRedelegationIxData,
};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType)]
pub enum State {
    Initialized,
    Redelegating,
    Completed,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankAccount)]
pub struct RedelegateState {
    pub is_initialized: bool,
    pub owner: Pubkey,
    pub state: State,
    pub current_validator: Pubkey,
    pub new_validator: Pubkey,
    pub stake_amount: u64,
    pub redelegation_timestamp: i64,
}

impl DataLen for RedelegateState {
    const LEN: usize = core::mem::size_of::<RedelegateState>();
}

impl Initialized for RedelegateState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl RedelegateState {
    pub const SEED: &'static str = "redelegate";

    pub fn validate_pda(bump: u8, pda: &Pubkey, owner: &Pubkey) -> Result<(), ProgramError> {
        let seed_with_bump = &[Self::SEED.as_bytes(), owner, &[bump]];
        let derived = pubkey::create_program_address(seed_with_bump, &crate::ID)?;
        if derived != *pda {
            return Err(ProgramError::InvalidSeeds);
        }
        Ok(())
    }

    pub fn start_redelegation(&mut self, ix_data: &StartRedelegationIxData) -> ProgramResult {
        self.new_validator = ix_data.new_validator;
        self.state = State::Redelegating;
        self.redelegation_timestamp = Clock::get()?.unix_timestamp;
        Ok(())
    }

    pub fn complete_redelegation(&mut self) -> ProgramResult {
        self.current_validator = self.new_validator;
        self.new_validator = Pubkey::default();
        self.state = State::Completed;
        self.redelegation_timestamp = 0;
        Ok(())
    }
}
