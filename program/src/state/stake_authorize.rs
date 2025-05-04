/*
#[cfg_attr(feature = "frozen-abi", derive(solana_frozen_abi_macro::AbiExample))]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Deserialize, serde_derive::Serialize)
)]

     */
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StakeAuthorize {
    Staker,
    Withdrawer,
}
