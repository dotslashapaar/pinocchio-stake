
use pinocchio::pubkey::Pubkey;
use crate::state::stake_authorize::StakeAuthorize;

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AuthorizeCheckedWithSeedArgs <'a>{
    pub stake_authorize: StakeAuthorize,
    pub authority_seed_len:u32,
    pub authority_seed: &'a str,
    pub authority_owner:Pubkey,
}
