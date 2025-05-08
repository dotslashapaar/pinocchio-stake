use pinocchio::pubkey::Pubkey;
use pinocchio_pubkey::pubkey;

pub const MAX_SIGNERS: usize = 32;
pub const FEATURE_STAKE_RAISE_MINIMUM_DELEGATION_TO_1_SOL: bool = false;
pub const PERPETUAL_NEW_WARMUP_COOLDOWN_RATE_EPOCH: Option<[u8; 8]> = Some(0u64.to_le_bytes());
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
pub const SYSVAR: Pubkey = pubkey!("Sysvar1111111111111111111111111111111111111");
pub const DEFAULT_WARMUP_COOLDOWN_RATE: f64 = 0.25;
pub const NEW_WARMUP_COOLDOWN_RATE: f64 = 0.09;
pub const CLOCK_ID: Pubkey = pubkey!("SysvarC1ock11111111111111111111111111111111");
