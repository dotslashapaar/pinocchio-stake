use pinocchio::pubkey::Pubkey;

#[repr(C)]
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub struct AuthorizedCheckedWithSeeds {
    pub stake_authorize: Pubkey,
    pub authority_seed: Pubkey,
    pub authority_owner:String,
}
