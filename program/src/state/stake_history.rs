use crate::declare_sysvar_id;
use pinocchio::pubkey::Pubkey;
use pinocchio::sysvars::clock::Epoch;
extern crate alloc;

//use {solana_sysvar_id::declare_sysvar_id, std::ops::Deref};
// These are substituted by the below
use core::ops::Deref;

/// A type that holds sysvar data and has an associated sysvar `Pubkey`.
pub trait SysvarId {
    /// The `Pubkey` of the sysvar.
    fn id() -> Pubkey;

    /// Returns `true` if the given pubkey is the program ID.
    fn check_id(pubkey: &Pubkey) -> bool;
}

/// Declares an ID that implements [`SysvarId`].

pub const MAX_ENTRIES: usize = 512; // it should never take as many as 512 epochs to warm up or cool down

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct StakeHistoryEntry {
    pub effective: [u8; 8],    // effective stake at this epoch
    pub activating: [u8; 8],   // sum of portion of stakes not fully warmed up
    pub deactivating: [u8; 8], // requested to be cooled down, not fully deactivated yet
}

impl StakeHistoryEntry {
    pub fn with_effective(effective: [u8; 8]) -> Self {
        Self {
            effective,
            ..Self::default()
        }
    }

    pub fn with_effective_and_activating(effective: [u8; 8], activating: [u8; 8]) -> Self {
        Self {
            effective,
            activating,
            ..Self::default()
        }
    }

    pub fn with_deactivating(deactivating: u64) -> Self {
        Self {
            effective: deactivating.to_le_bytes(),
            deactivating: deactivating.to_le_bytes(),
            ..Self::default()
        }
    }
}

impl core::ops::Add for StakeHistoryEntry {
    type Output = StakeHistoryEntry;
    fn add(self, rhs: StakeHistoryEntry) -> Self::Output {
        let effective = u64::from_le_bytes(self.effective);
        let activating = u64::from_le_bytes(self.activating);
        let deactivating = u64::from_le_bytes(self.deactivating);
        Self {
            effective: effective
                .saturating_add(u64::from_le_bytes(rhs.effective))
                .to_be_bytes(),
            activating: activating
                .saturating_add(u64::from_le_bytes(rhs.activating))
                .to_be_bytes(),
            deactivating: deactivating
                .saturating_add(u64::from_le_bytes(rhs.deactivating))
                .to_be_bytes(),
        }
    }
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct StakeHistory(alloc::vec::Vec<(Epoch, StakeHistoryEntry)>);

declare_sysvar_id!("SysvarStakeHistory1111111111111111111111111", StakeHistory);

impl StakeHistory {
    pub fn get(&self, epoch: Epoch) -> Option<&StakeHistoryEntry> {
        self.binary_search_by(|probe| epoch.cmp(&probe.0))
            .ok()
            .map(|index| &self[index].1)
    }

    pub fn add(&mut self, epoch: Epoch, entry: StakeHistoryEntry) {
        match self.binary_search_by(|probe| epoch.cmp(&probe.0)) {
            Ok(index) => (self.0)[index] = (epoch, entry),
            Err(index) => (self.0).insert(index, (epoch, entry)),
        }
        (self.0).truncate(MAX_ENTRIES);
    }
}

#[deprecated(
    since = "2.2.0",
    note = "Use solana_stake_interface::stake_history instead"
)]
impl StakeHistory {
    // override
    fn size_of() -> usize {
        // hard-coded so that we don't have to construct an empty
        16392 // golden, update if MAX_ENTRIES changes
    }
}

impl Deref for StakeHistory {
    type Target = alloc::vec::Vec<(Epoch, StakeHistoryEntry)>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait StakeHistoryGetEntry {
    fn get_entry(&self, epoch: Epoch) -> Option<StakeHistoryEntry>;
}

impl StakeHistoryGetEntry for StakeHistory {
    fn get_entry(&self, epoch: Epoch) -> Option<StakeHistoryEntry> {
        self.binary_search_by(|probe| epoch.cmp(&probe.0))
            .ok()
            .map(|index| self[index].1.clone())
    }
}
