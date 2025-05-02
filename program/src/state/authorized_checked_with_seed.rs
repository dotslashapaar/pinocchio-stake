#![no_std]
extern crate alloc;

use alloc::string::String;
use pinocchio::pubkey::Pubkey;
use crate::state::stake_authorize::StakeAuthorize;

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AuthorizeCheckedWithSeedArgs {
    pub stake_authorize: StakeAuthorize,
    pub authority_seed: String,
    pub authority_owner:Pubkey,
}
