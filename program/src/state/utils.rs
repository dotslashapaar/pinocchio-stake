use pinocchio::{
    account_info::{AccountInfo, Ref},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult, SUCCESS,
};

extern crate alloc;
use super::{
    get_stake_state, set_stake_state, Meta, StakeAuthorize, StakeHistorySysvar, StakeStateV2,
    DEFAULT_WARMUP_COOLDOWN_RATE,
};
use crate::{
    consts::{
        FEATURE_STAKE_RAISE_MINIMUM_DELEGATION_TO_1_SOL, LAMPORTS_PER_SOL, MAX_SIGNERS,
        NEW_WARMUP_COOLDOWN_RATE, SYSVAR,
    },
    helpers::MergeKind,
};
use alloc::boxed::Box;
use core::cell::UnsafeCell;

pub trait DataLen {
    const LEN: usize;
}

pub trait Initialized {
    fn is_initialized(&self) -> bool;
}

#[inline(always)]
pub unsafe fn load_acc<T: DataLen + Initialized>(bytes: &[u8]) -> Result<&T, ProgramError> {
    load_acc_unchecked::<T>(bytes).and_then(|acc| {
        if acc.is_initialized() {
            Ok(acc)
        } else {
            Err(ProgramError::UninitializedAccount)
        }
    })
}

#[inline(always)]
pub unsafe fn load_acc_unchecked<T: DataLen>(bytes: &[u8]) -> Result<&T, ProgramError> {
    if bytes.len() != T::LEN {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(&*(bytes.as_ptr() as *const T))
}

#[inline(always)]
pub unsafe fn load_acc_mut<T: DataLen + Initialized>(
    bytes: &mut [u8],
) -> Result<&mut T, ProgramError> {
    load_acc_mut_unchecked::<T>(bytes).and_then(|acc| {
        if acc.is_initialized() {
            Ok(acc)
        } else {
            Err(ProgramError::UninitializedAccount)
        }
    })
}

#[inline(always)]
pub unsafe fn load_acc_mut_unchecked<T: DataLen>(bytes: &mut [u8]) -> Result<&mut T, ProgramError> {
    if bytes.len() != T::LEN {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(&mut *(bytes.as_mut_ptr() as *mut T))
}

#[inline(always)]
pub unsafe fn load_ix_data<T: DataLen>(bytes: &[u8]) -> Result<&T, ProgramError> {
    if bytes.len() != T::LEN {
        return Err(ProgramError::InvalidInstructionData.into());
    }
    Ok(&*(bytes.as_ptr() as *const T))
}

pub unsafe fn to_bytes<T: DataLen>(data: &T) -> &[u8] {
    core::slice::from_raw_parts(data as *const T as *const u8, T::LEN)
}

pub unsafe fn to_mut_bytes<T: DataLen>(data: &mut T) -> &mut [u8] {
    core::slice::from_raw_parts_mut(data as *mut T as *mut u8, T::LEN)
}

//---------- Stake Program Utils -------------

pub fn collect_signers(
    accounts: &[AccountInfo],
    signers_arr: &mut [Pubkey; MAX_SIGNERS],
) -> Result<usize, ProgramError> {
    let mut signer_len = 0;

    for account in accounts {
        if account.is_signer() {
            if signer_len >= MAX_SIGNERS {
                return Err(ProgramError::AccountDataTooSmall);
            }
            signers_arr[signer_len] = *account.key();
            signer_len += 1;
        }
    }

    Ok(signer_len)
}

pub fn next_account_info<'a, I: Iterator<Item = &'a AccountInfo>>(
    iter: &mut I,
) -> Result<&'a AccountInfo, ProgramError> {
    iter.next().ok_or(ProgramError::NotEnoughAccountKeys)
}

#[macro_export]
macro_rules! impl_sysvar_id {
    ($type:ty) => {
        impl $crate::state::stake_history::SysvarId for $type {
            fn id() -> Pubkey {
                id()
            }

            fn check_id(pubkey: &Pubkey) -> bool {
                check_id(pubkey)
            }
        }
    };
}

#[macro_export]
macro_rules! declare_sysvar_id(
    ($name:expr, $type:ty) => (
        pinocchio_pubkey::declare_id!($name);
        $crate::impl_sysvar_id!($type);
    )
);

/// After calling `validate_split_amount()`, this struct contains calculated
/// values that are used by the caller.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct ValidatedSplitInfo {
    pub source_remaining_balance: u64,
    pub destination_rent_exempt_reserve: u64,
}

/// Ensure the split amount is valid.  This checks the source and destination
/// accounts meet the minimum balance requirements, which is the rent exempt
/// reserve plus the minimum stake delegation, and that the source account has
/// enough lamports for the request split amount.  If not, return an error.
pub(crate) fn validate_split_amount(
    source_lamports: u64,
    destination_lamports: u64,
    split_lamports: u64,
    source_meta: &Meta,
    destination_data_len: usize,
    additional_required_lamports: u64,
    source_is_active: bool,
) -> Result<ValidatedSplitInfo, ProgramError> {
    // Split amount has to be something
    if split_lamports == 0 {
        return Err(ProgramError::InsufficientFunds);
    }

    // Obviously cannot split more than what the source account has
    if split_lamports > source_lamports {
        return Err(ProgramError::InsufficientFunds);
    }

    // Verify that the source account still has enough lamports left after
    // splitting: EITHER at least the minimum balance, OR zero (in this case the
    // source account is transferring all lamports to new destination account,
    // and the source account will be closed)
    let source_minimum_balance = u64::from_le_bytes(source_meta.rent_exempt_reserve)
        .saturating_add(additional_required_lamports);
    let source_remaining_balance = source_lamports.saturating_sub(split_lamports);
    if source_remaining_balance == 0 {
        // full amount is a withdrawal
        // nothing to do here
    } else if source_remaining_balance < source_minimum_balance {
        // the remaining balance is too low to do the split
        return Err(ProgramError::InsufficientFunds);
    } else {
        // all clear!
        // nothing to do here
    }

    let rent = Rent::get()?;
    let destination_rent_exempt_reserve = rent.minimum_balance(destination_data_len);

    // If the source is active stake, one of these criteria must be met:
    // 1. the destination account must be prefunded with at least the rent-exempt
    //    reserve, or
    // 2. the split must consume 100% of the source
    if source_is_active
        && source_remaining_balance != 0
        && destination_lamports < destination_rent_exempt_reserve
    {
        return Err(ProgramError::InsufficientFunds);
    }

    // Verify the destination account meets the minimum balance requirements
    // This must handle:
    // 1. The destination account having a different rent exempt reserve due to data
    //    size changes
    // 2. The destination account being prefunded, which would lower the minimum
    //    split amount
    let destination_minimum_balance =
        destination_rent_exempt_reserve.saturating_add(additional_required_lamports);
    let destination_balance_deficit =
        destination_minimum_balance.saturating_sub(destination_lamports);
    if split_lamports < destination_balance_deficit {
        return Err(ProgramError::InsufficientFunds);
    }

    Ok(ValidatedSplitInfo {
        source_remaining_balance,
        destination_rent_exempt_reserve,
    })
}

//-------------- Solana Program Sysvar Copies ---------------

//---------------- This Get Sysvar was assisted by AI, needs to be checked ----------------------
//For this syscall mock, unlike solana program we use single thread to mantain the no_std enviorement
//Defining a generic Lazy<T> struct with interior mutability
pub struct Lazy<T> {
    value: UnsafeCell<Option<T>>,
}

impl<T> Lazy<T> {
    pub const fn new() -> Self {
        Self {
            value: UnsafeCell::new(None),
        }
    }

    pub fn get_or_init<F: FnOnce() -> T>(&self, init: F) -> &T {
        // SAFETY: Only safe because Solana programs are single-threaded.
        // So its ok to get mutable access (even though `self` is shared!)
        unsafe {
            let value = &mut *self.value.get();
            if value.is_none() {
                *value = Some(init());
            }
            value.as_ref().unwrap()
        }
    }
}

static SYSCALL_STUBS: Lazy<Box<dyn SyscallStubs>> = Lazy::new();

unsafe impl<T> Sync for Lazy<T> {} //although this is telling that is available for multithreading, we know it wont happen

/// Builtin return values occupy the upper 32 bits
const BUILTIN_BIT_SHIFT: usize = 32;
macro_rules! to_builtin {
    ($error:expr) => {
        ($error as u64) << BUILTIN_BIT_SHIFT
    };
}

pub const UNSUPPORTED_SYSVAR: u64 = to_builtin!(17);

pub trait SyscallStubs: Sync + Send {
    fn sol_get_sysvar(
        &self,
        _sysvar_id_addr: *const u8,
        _var_addr: *mut u8,
        _offset: u64,
        _length: u64,
    ) -> u64 {
        UNSUPPORTED_SYSVAR
    }
}

pub struct DefaultSyscallStubs {}

impl SyscallStubs for DefaultSyscallStubs {}

#[allow(dead_code)]
pub(crate) fn sol_get_sysvar(
    sysvar_id_addr: *const u8,
    var_addr: *mut u8,
    offset: u64,
    length: u64,
) -> u64 {
    SYSCALL_STUBS
        .get_or_init(|| Box::new(DefaultSyscallStubs {}))
        .sol_get_sysvar(sysvar_id_addr, var_addr, offset, length)
}

//---------------- End of AI assistance ----------------------

/// Handler for retrieving a slice of sysvar data from the `sol_get_sysvar`
/// syscall.
pub fn get_sysvar(
    dst: &mut [u8],
    sysvar_id: &Pubkey,
    offset: u64,
    length: u64,
) -> Result<(), ProgramError> {
    // Check that the provided destination buffer is large enough to hold the
    // requested data.
    if dst.len() < length as usize {
        return Err(ProgramError::InvalidArgument);
    }

    let sysvar_id = sysvar_id as *const _ as *const u8;
    let var_addr = dst as *mut _ as *mut u8;

    //if on Solana call the actual syscall
    #[cfg(target_os = "solana")]
    let result =
        unsafe { pinocchio::syscalls::sol_get_sysvar(sysvar_id, var_addr, offset, length) };

    //if not on chain use the mock
    #[cfg(not(target_os = "solana"))]
    let result = sol_get_sysvar(sysvar_id, var_addr, offset, length);

    match result {
        SUCCESS => Ok(()),
        e => Err(e.into()),
    }
}

pub fn to_program_error(e: ProgramError) -> ProgramError {
    ProgramError::try_from(e).unwrap_or(ProgramError::InvalidAccountData)
}

#[inline(always)]
pub fn get_minimum_delegation() -> u64 {
    if FEATURE_STAKE_RAISE_MINIMUM_DELEGATION_TO_1_SOL {
        const MINIMUM_DELEGATION_SOL: u64 = 1;
        MINIMUM_DELEGATION_SOL * LAMPORTS_PER_SOL
    } else {
        1
    }
}

pub fn do_authorize(
    stake_account_info: &AccountInfo,
    signers: &[Pubkey],
    new_authority: &Pubkey,
    authority_type: StakeAuthorize,
    custodian: Option<&Pubkey>,
    clock: Clock,
) -> ProgramResult {
    let mut stake_account: pinocchio::account_info::RefMut<'_, StakeStateV2> =
        try_get_stake_state_mut(stake_account_info)?;
    match *get_stake_state(stake_account_info)? {
        StakeStateV2::Initialized(mut meta) => {
            meta.authorized
                .authorize(
                    signers,
                    new_authority,
                    authority_type,
                    Some((&meta.lockup, &clock, custodian)),
                )
                .map_err(to_program_error)?;
            *stake_account = StakeStateV2::Initialized(meta);
            Ok(())
        }
        StakeStateV2::Stake(mut meta, stake, stake_flags) => {
            meta.authorized
                .authorize(
                    signers,
                    new_authority,
                    authority_type,
                    Some((&meta.lockup, &clock, custodian)),
                )
                .map_err(to_program_error)?;

            *stake_account = StakeStateV2::Stake(meta, stake, stake_flags);
            Ok(())
        }
        _ => Err(ProgramError::InvalidAccountData),
    }
}

// Means that no more than RATE of current effective stake may be added or subtracted per
// epoch.

pub fn warmup_cooldown_rate(
    current_epoch: [u8; 8],
    new_rate_activation_epoch: Option<[u8; 8]>,
) -> f64 {
    let current = bytes_to_u64(current_epoch);
    let activation = new_rate_activation_epoch
        .map(bytes_to_u64)
        .unwrap_or(u64::MAX);

    if current < activation {
        DEFAULT_WARMUP_COOLDOWN_RATE
    } else {
        NEW_WARMUP_COOLDOWN_RATE
    }
}

pub fn add_le_bytes(lhs: [u8; 8], rhs: [u8; 8]) -> [u8; 8] {
    u64::from_le_bytes(lhs)
        .saturating_add(u64::from_le_bytes(rhs))
        .to_le_bytes()
}

pub fn bytes_to_u64(bytes: [u8; 8]) -> u64 {
    u64::from_le_bytes(bytes)
}

// MoveStake, MoveLamports, Withdraw, and AuthorizeWithSeed assemble signers explicitly
pub fn collect_signers_checked<'a>(
    authority_info: Option<&'a AccountInfo>,
    custodian_info: Option<&'a AccountInfo>,
) -> Result<([Pubkey; MAX_SIGNERS], Option<&'a Pubkey>, usize), ProgramError> {
    let mut signers: [Pubkey; MAX_SIGNERS] = Default::default();
    let mut signers_count = 0;

    if let Some(authority_info) = authority_info {
        add_signer(&mut signers, &mut signers_count, authority_info)?;
    }

    let custodian = if let Some(custodian_info) = custodian_info {
        add_signer(&mut signers, &mut signers_count, custodian_info)?;
        Some(custodian_info.key())
    } else {
        None
    };

    Ok((signers, custodian, signers_count))
}

pub fn add_signer(
    signers: &mut [Pubkey; MAX_SIGNERS],
    signers_count: &mut usize,
    account_info: &AccountInfo,
) -> Result<(), ProgramError> {
    if *signers_count >= MAX_SIGNERS {
        return Err(ProgramError::MaxAccountsDataAllocationsExceeded);
    }
    if !account_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    signers[*signers_count] = *account_info.key();
    *signers_count += 1;
    Ok(())
}

pub fn move_stake_or_lamports_shared_checks(
    source_stake_account_info: &AccountInfo,
    lamports: u64,
    destination_stake_account_info: &AccountInfo,
    stake_authority_info: &AccountInfo,
) -> Result<(MergeKind, MergeKind), ProgramError> {
    // authority must sign
    let (signers, _, _) = collect_signers_checked(Some(stake_authority_info), None)?;

    // confirm not the same account
    if *source_stake_account_info.key() == *destination_stake_account_info.key() {
        return Err(ProgramError::InvalidInstructionData);
    }

    // source and destination must be writable
    // runtime guards against unowned writes, but MoveStake and MoveLamports are defined by SIMD
    // we check explicitly to avoid any possibility of a successful no-op that never attempts to write
    if !source_stake_account_info.is_writable() || !destination_stake_account_info.is_writable() {
        return Err(ProgramError::InvalidInstructionData);
    }

    // must move something
    if lamports == 0 {
        return Err(ProgramError::InvalidArgument);
    }

    let clock = Clock::get()?;
    let stake_history = StakeHistorySysvar(clock.epoch);

    // get_if_mergeable ensures accounts are not partly activated or in any form of deactivating
    // we still need to exclude activating state ourselves
    let source_merge_kind = MergeKind::get_if_mergeable(
        &*get_stake_state(source_stake_account_info)?,
        source_stake_account_info.lamports(),
        &clock,
        &stake_history,
    )?;

    // Authorized staker is allowed to move stake
    source_merge_kind
        .meta()
        .authorized
        .check(&signers, StakeAuthorize::Staker)
        .map_err(to_program_error)?;

    // same transient assurance as with source
    let destination_merge_kind = MergeKind::get_if_mergeable(
        &*get_stake_state(destination_stake_account_info)?,
        destination_stake_account_info.lamports(),
        &clock,
        &stake_history,
    )?;

    // ensure all authorities match and lockups match if lockup is in force
    MergeKind::metas_can_merge(
        source_merge_kind.meta(),
        destination_merge_kind.meta(),
        &clock,
    )?;

    Ok((source_merge_kind, destination_merge_kind))
}
