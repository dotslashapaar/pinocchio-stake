#![no_std]
extern crate alloc;

use alloc::string::String;
use pinocchio::pubkey::Pubkey;

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StakeAuthorize {
    Staker,
    Withdrawer,
}
#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AuthorizeCheckedWithSeedArgs {
    pub stake_authorize: StakeAuthorize,
    pub authority_seed: String,
    pub authority_owner:Pubkey,
}
