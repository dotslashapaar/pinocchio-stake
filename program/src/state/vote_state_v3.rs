use pinocchio::{
    account_info::{ AccountInfo, Ref },
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvars::{ clock::{ Clock, Epoch, Slot, UnixTimestamp }, rent::Rent },
};
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use crate::{consts::{INITIAL_LOCKOUT, MAX_LOCKOUT_HISTORY}, state::Hash};

// available in /solana-vote-interface-2.2.4/src/state/vote_state_v3.rs
#[repr(C)]
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct VoteState {
    /// the node that votes in this account
    pub node_pubkey: Pubkey,

    /// the signer for withdrawals
    pub authorized_withdrawer: Pubkey,
    /// percentage (0-100) that represents what part of a rewards
    ///  payout should be given to this VoteAccount
    pub commission: u8,

    pub votes: VecDeque<LandedVote>,

    // This usually the last Lockout which was popped from self.votes.
    // However, it can be arbitrary slot, when being used inside Tower
    pub root_slot: Option<Slot>,

    /// the signer for vote transactions
    pub authorized_voters: AuthorizedVoters,

    /// history of prior authorized voters and the epochs for which
    /// they were set, the bottom end of the range is inclusive,
    /// the top of the range is exclusive
    pub prior_voters: CircBuf<(Pubkey, Epoch, Epoch)>,

    /// history of how many credits earned by the end of each epoch
    ///  each tuple is (Epoch, credits, prev_credits)
    pub epoch_credits: Vec<(Epoch, u64, u64)>,

    /// most recent timestamp submitted with a vote
    pub last_timestamp: BlockTimestamp,
}

impl VoteState {
    pub fn new(vote_init: &VoteInit, clock: &Clock) -> Self {
        Self {
            node_pubkey: vote_init.node_pubkey,
            authorized_voters: AuthorizedVoters::new(clock.epoch, vote_init.authorized_voter),
            authorized_withdrawer: vote_init.authorized_withdrawer,
            commission: vote_init.commission,
            ..VoteState::default()
        }
    }

    pub fn get_authorized_voter(&self, epoch: Epoch) -> Option<Pubkey> {
        self.authorized_voters.get_authorized_voter(epoch)
    }

    pub fn authorized_voters(&self) -> &AuthorizedVoters {
        &self.authorized_voters
    }

    pub fn prior_voters(&mut self) -> &CircBuf<(Pubkey, Epoch, Epoch)> {
        &self.prior_voters
    }

    pub fn get_rent_exempt_reserve(rent: &Rent) -> u64 {
        rent.minimum_balance(VoteState::size_of())
    }

    /// Upper limit on the size of the Vote State
    /// when votes.len() is MAX_LOCKOUT_HISTORY.
    pub const fn size_of() -> usize {
        3762 // see test_vote_state_size_of.
    }

    #[inline]
    pub fn from_account_info(account_info: &AccountInfo) -> Result<Ref<VoteState>, ProgramError> {
        if account_info.data_len() != Self::size_of() {
            return Err(ProgramError::InvalidAccountData);
        }
        let data = account_info.try_borrow_data()?;
        Ok(Ref::map(data, |data| unsafe { Self::from_bytes(data) }))
    }

    #[inline(always)]
    pub unsafe fn from_bytes(bytes: &[u8]) -> &Self {
        &*(bytes.as_ptr() as *const Self)
    }

    /// Number of "credits" owed to this account from the mining pool. Submit this
    /// VoteState to the Rewards program to trade credits for lamports.
    pub fn credits(&self) -> u64 {
        if self.epoch_credits.is_empty() {
            0
        } else {
            self.epoch_credits.last().unwrap().1
        }
    }
}

// -------------solana-vote-interface/src/state/mod.rs------------------
/// Vote state

use super::AuthorizedVoters;


#[repr(C)]
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct Vote {
    /// A stack of votes starting with the oldest vote
    pub slots: Vec<Slot>,
    /// signature of the bank's state at the last slot
    pub hash: Hash,
    /// processing timestamp of last slot
    pub timestamp: Option<UnixTimestamp>,
}

impl Vote {
    pub fn new(slots: Vec<Slot>, hash: Hash) -> Self {
        Self {
            slots,
            hash,
            timestamp: None,
        }
    }

    pub fn last_voted_slot(&self) -> Option<Slot> {
        self.slots.last().copied()
    }
}

#[repr(C)]
#[derive(Default, Debug, PartialEq, Eq, Copy, Clone)]
pub struct Lockout {
    slot: Slot,
    confirmation_count: u32,
}

impl Lockout {
    pub fn new(slot: Slot) -> Self {
        Self::new_with_confirmation_count(slot, 1)
    }

    pub fn new_with_confirmation_count(slot: Slot, confirmation_count: u32) -> Self {
        Self {
            slot,
            confirmation_count,
        }
    }

    // The number of slots for which this vote is locked
    pub fn lockout(&self) -> u64 {
        (INITIAL_LOCKOUT as u64).wrapping_pow(
            core::cmp::min(self.confirmation_count(), MAX_LOCKOUT_HISTORY as u32)
        )
    }

    // The last slot at which a vote is still locked out. Validators should not
    // vote on a slot in another fork which is less than or equal to this slot
    // to avoid having their stake slashed.
    pub fn last_locked_out_slot(&self) -> Slot {
        self.slot.saturating_add(self.lockout())
    }

    pub fn is_locked_out_at_slot(&self, slot: Slot) -> bool {
        self.last_locked_out_slot() >= slot
    }

    pub fn slot(&self) -> Slot {
        self.slot
    }

    pub fn confirmation_count(&self) -> u32 {
        self.confirmation_count
    }

    pub fn increase_confirmation_count(&mut self, by: u32) {
        self.confirmation_count = self.confirmation_count.saturating_add(by);
    }
}

#[repr(C)]
#[derive(Default, Debug, PartialEq, Eq, Copy, Clone)]
pub struct LandedVote {
    // Latency is the difference in slot number between the slot that was voted on (lockout.slot) and the slot in
    // which the vote that added this Lockout landed.  For votes which were cast before versions of the validator
    // software which recorded vote latencies, latency is recorded as 0.
    pub latency: u8,
    pub lockout: Lockout,
}

impl LandedVote {
    pub fn slot(&self) -> Slot {
        self.lockout.slot
    }

    pub fn confirmation_count(&self) -> u32 {
        self.lockout.confirmation_count
    }
}

impl From<LandedVote> for Lockout {
    fn from(landed_vote: LandedVote) -> Self {
        landed_vote.lockout
    }
}

impl From<Lockout> for LandedVote {
    fn from(lockout: Lockout) -> Self {
        Self {
            latency: 0,
            lockout,
        }
    }
}

#[repr(C)]
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct VoteStateUpdate {
    /// The proposed tower
    pub lockouts: VecDeque<Lockout>,
    /// The proposed root
    pub root: Option<Slot>,
    /// signature of the bank's state at the last slot
    pub hash: Hash,
    /// processing timestamp of last slot
    pub timestamp: Option<UnixTimestamp>,
}

impl From<Vec<(Slot, u32)>> for VoteStateUpdate {
    fn from(recent_slots: Vec<(Slot, u32)>) -> Self {
        let lockouts: VecDeque<Lockout> = recent_slots
            .into_iter()
            .map(|(slot, confirmation_count)| {
                Lockout::new_with_confirmation_count(slot, confirmation_count)
            })
            .collect();
        Self {
            lockouts,
            root: None,
            hash: Hash::default(),
            timestamp: None,
        }
    }
}

impl VoteStateUpdate {
    pub fn new(lockouts: VecDeque<Lockout>, root: Option<Slot>, hash: Hash) -> Self {
        Self {
            lockouts,
            root,
            hash,
            timestamp: None,
        }
    }

    pub fn slots(&self) -> Vec<Slot> {
        self.lockouts
            .iter()
            .map(|lockout| lockout.slot())
            .collect()
    }

    pub fn last_voted_slot(&self) -> Option<Slot> {
        self.lockouts.back().map(|l| l.slot())
    }
}

#[repr(C)]
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct TowerSync {
    /// The proposed tower
    pub lockouts: VecDeque<Lockout>,
    /// The proposed root
    pub root: Option<Slot>,
    /// signature of the bank's state at the last slot
    pub hash: Hash,
    /// processing timestamp of last slot
    pub timestamp: Option<UnixTimestamp>,
    /// the unique identifier for the chain up to and
    /// including this block. Does not require replaying
    /// in order to compute.
    pub block_id: Hash,
}

impl From<Vec<(Slot, u32)>> for TowerSync {
    fn from(recent_slots: Vec<(Slot, u32)>) -> Self {
        let lockouts: VecDeque<Lockout> = recent_slots
            .into_iter()
            .map(|(slot, confirmation_count)| {
                Lockout::new_with_confirmation_count(slot, confirmation_count)
            })
            .collect();
        Self {
            lockouts,
            root: None,
            hash: Hash::default(),
            timestamp: None,
            block_id: Hash::default(),
        }
    }
}

impl TowerSync {
    pub fn new(
        lockouts: VecDeque<Lockout>,
        root: Option<Slot>,
        hash: Hash,
        block_id: Hash
    ) -> Self {
        Self {
            lockouts,
            root,
            hash,
            timestamp: None,
            block_id,
        }
    }

    /// Creates a tower with consecutive votes for `slot - MAX_LOCKOUT_HISTORY + 1` to `slot` inclusive.
    /// If `slot >= MAX_LOCKOUT_HISTORY`, sets the root to `(slot - MAX_LOCKOUT_HISTORY)`
    /// Sets the hash to `hash` and leaves `block_id` unset.
    pub fn new_from_slot(slot: Slot, hash: Hash) -> Self {
        let lowest_slot = slot.saturating_add(1).saturating_sub(MAX_LOCKOUT_HISTORY as u64);
        let slots: Vec<_> = (lowest_slot..slot.saturating_add(1)).collect();
        Self::new_from_slots(
            slots,
            hash,
            (lowest_slot > 0).then(|| lowest_slot.saturating_sub(1))
        )
    }

    /// Creates a tower with consecutive confirmation for `slots`
    pub fn new_from_slots(slots: Vec<Slot>, hash: Hash, root: Option<Slot>) -> Self {
        let lockouts: VecDeque<Lockout> = slots
            .into_iter()
            .rev()
            .enumerate()
            .map(|(cc, s)| Lockout::new_with_confirmation_count(s, cc.saturating_add(1) as u32))
            .rev()
            .collect();
        Self {
            lockouts,
            hash,
            root,
            timestamp: None,
            block_id: Hash::default(),
        }
    }

    pub fn slots(&self) -> Vec<Slot> {
        self.lockouts
            .iter()
            .map(|lockout| lockout.slot())
            .collect()
    }

    pub fn last_voted_slot(&self) -> Option<Slot> {
        self.lockouts.back().map(|l| l.slot())
    }
}

#[repr(C)]
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub struct VoteInit {
    pub node_pubkey: Pubkey,
    pub authorized_voter: Pubkey,
    pub authorized_withdrawer: Pubkey,
    pub commission: u8,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum VoteAuthorize {
    Voter,
    Withdrawer,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct VoteAuthorizeWithSeedArgs {
    pub authorization_type: VoteAuthorize,
    pub current_authority_derived_key_owner: Pubkey,
    pub current_authority_derived_key_seed: alloc::vec::Vec<u8>, // changed String to Vec<u8>
    pub new_authority: Pubkey,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct VoteAuthorizeCheckedWithSeedArgs {
    pub authorization_type: VoteAuthorize,
    pub current_authority_derived_key_owner: Pubkey,
    pub current_authority_derived_key_seed: alloc::vec::Vec<u8>, // changed String to Vec<u8>
}

#[repr(C)]
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct BlockTimestamp {
    pub slot: Slot,
    pub timestamp: UnixTimestamp,
}

// this is how many epochs a voter can be remembered for slashing
const MAX_ITEMS: usize = 32;

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CircBuf<I> {
    buf: [I; MAX_ITEMS],
    /// next pointer
    idx: usize,
    is_empty: bool,
}

impl<I: Default + Copy> Default for CircBuf<I> {
    fn default() -> Self {
        Self {
            buf: [I::default(); MAX_ITEMS],
            idx: MAX_ITEMS.checked_sub(1).expect("`MAX_ITEMS` should be positive"),
            is_empty: true,
        }
    }
}

impl<I> CircBuf<I> {
    pub fn append(&mut self, item: I) {
        // remember prior delegate and when we switched, to support later slashing
        self.idx = self.idx
            .checked_add(1)
            .and_then(|idx| idx.checked_rem(MAX_ITEMS))
            .expect("`self.idx` should be < `MAX_ITEMS` which should be non-zero");

        self.buf[self.idx] = item;
        self.is_empty = false;
    }

    pub fn buf(&self) -> &[I; MAX_ITEMS] {
        &self.buf
    }

    pub fn last(&self) -> Option<&I> {
        if !self.is_empty { self.buf.get(self.idx) } else { None }
    }
}

// serde conversion for VoteStateUpdate and TowerSync -----------------------

// #[cfg(feature = "serde")]
// pub mod serde_compact_vote_state_update {
//     use ::{
//         super::*,
//         crate::state::Lockout,
//         serde::{ Deserialize, Deserializer, Serialize, Serializer },
//         solana_serde_varint as serde_varint,
//         solana_short_vec as short_vec,
//     };

//     #[cfg_attr(feature = "frozen-abi", derive(AbiExample))]
//     #[derive(serde_derive::Deserialize, serde_derive::Serialize)]
//     struct LockoutOffset {
//         #[serde(with = "serde_varint")]
//         offset: Slot,
//         confirmation_count: u8,
//     }

//     #[derive(serde_derive::Deserialize, serde_derive::Serialize)]
//     struct CompactVoteStateUpdate {
//         root: Slot,
//         #[serde(with = "short_vec")]
//         lockout_offsets: Vec<LockoutOffset>,
//         hash: Hash,
//         timestamp: Option<UnixTimestamp>,
//     }

//     pub fn serialize<S>(
//         vote_state_update: &VoteStateUpdate,
//         serializer: S
//     ) -> Result<S::Ok, S::Error>
//         where S: Serializer
//     {
//         let lockout_offsets = vote_state_update.lockouts
//             .iter()
//             .scan(vote_state_update.root.unwrap_or_default(), |slot, lockout| {
//                 let Some(offset) = lockout.slot().checked_sub(*slot) else {
//                     return Some(Err(serde::ser::Error::custom("Invalid vote lockout")));
//                 };
//                 let Ok(confirmation_count) = u8::try_from(lockout.confirmation_count()) else {
//                     return Some(Err(serde::ser::Error::custom("Invalid confirmation count")));
//                 };
//                 let lockout_offset = LockoutOffset {
//                     offset,
//                     confirmation_count,
//                 };
//                 *slot = lockout.slot();
//                 Some(Ok(lockout_offset))
//             });
//         let compact_vote_state_update = CompactVoteStateUpdate {
//             root: vote_state_update.root.unwrap_or(Slot::MAX),
//             lockout_offsets: lockout_offsets.collect::<Result<_, _>>()?,
//             hash: vote_state_update.hash,
//             timestamp: vote_state_update.timestamp,
//         };
//         compact_vote_state_update.serialize(serializer)
//     }

//     pub fn deserialize<'de, D>(deserializer: D) -> Result<VoteStateUpdate, D::Error>
//         where D: Deserializer<'de>
//     {
//         let CompactVoteStateUpdate { root, lockout_offsets, hash, timestamp } =
//             CompactVoteStateUpdate::deserialize(deserializer)?;
//         let root = (root != Slot::MAX).then_some(root);
//         let lockouts = lockout_offsets
//             .iter()
//             .scan(root.unwrap_or_default(), |slot, lockout_offset| {
//                 *slot = match slot.checked_add(lockout_offset.offset) {
//                     None => {
//                         return Some(Err(serde::de::Error::custom("Invalid lockout offset")));
//                     }
//                     Some(slot) => slot,
//                 };
//                 let lockout = Lockout::new_with_confirmation_count(
//                     *slot,
//                     u32::from(lockout_offset.confirmation_count)
//                 );
//                 Some(Ok(lockout))
//             });
//         Ok(VoteStateUpdate {
//             root,
//             lockouts: lockouts.collect::<Result<_, _>>()?,
//             hash,
//             timestamp,
//         })
//     }
// }

// #[cfg(feature = "serde")]
// pub mod serde_tower_sync {
//     use ::{
//         super::*,
//         crate::state::Lockout,
//         serde::{ Deserialize, Deserializer, Serialize, Serializer },
//         solana_serde_varint as serde_varint,
//         solana_short_vec as short_vec,
//     };

//     #[cfg_attr(feature = "frozen-abi", derive(AbiExample))]
//     #[derive(serde_derive::Deserialize, serde_derive::Serialize)]
//     struct LockoutOffset {
//         #[serde(with = "serde_varint")]
//         offset: Slot,
//         confirmation_count: u8,
//     }

//     #[derive(serde_derive::Deserialize, serde_derive::Serialize)]
//     struct CompactTowerSync {
//         root: Slot,
//         #[serde(with = "short_vec")]
//         lockout_offsets: Vec<LockoutOffset>,
//         hash: Hash,
//         timestamp: Option<UnixTimestamp>,
//         block_id: Hash,
//     }

//     pub fn serialize<S>(tower_sync: &TowerSync, serializer: S) -> Result<S::Ok, S::Error>
//         where S: Serializer
//     {
//         let lockout_offsets = tower_sync.lockouts
//             .iter()
//             .scan(tower_sync.root.unwrap_or_default(), |slot, lockout| {
//                 let Some(offset) = lockout.slot().checked_sub(*slot) else {
//                     return Some(Err(serde::ser::Error::custom("Invalid vote lockout")));
//                 };
//                 let Ok(confirmation_count) = u8::try_from(lockout.confirmation_count()) else {
//                     return Some(Err(serde::ser::Error::custom("Invalid confirmation count")));
//                 };
//                 let lockout_offset = LockoutOffset {
//                     offset,
//                     confirmation_count,
//                 };
//                 *slot = lockout.slot();
//                 Some(Ok(lockout_offset))
//             });
//         let compact_tower_sync = CompactTowerSync {
//             root: tower_sync.root.unwrap_or(Slot::MAX),
//             lockout_offsets: lockout_offsets.collect::<Result<_, _>>()?,
//             hash: tower_sync.hash,
//             timestamp: tower_sync.timestamp,
//             block_id: tower_sync.block_id,
//         };
//         compact_tower_sync.serialize(serializer)
//     }

//     pub fn deserialize<'de, D>(deserializer: D) -> Result<TowerSync, D::Error>
//         where D: Deserializer<'de>
//     {
//         let CompactTowerSync { root, lockout_offsets, hash, timestamp, block_id } =
//             CompactTowerSync::deserialize(deserializer)?;
//         let root = (root != Slot::MAX).then_some(root);
//         let lockouts = lockout_offsets
//             .iter()
//             .scan(root.unwrap_or_default(), |slot, lockout_offset| {
//                 *slot = match slot.checked_add(lockout_offset.offset) {
//                     None => {
//                         return Some(Err(serde::de::Error::custom("Invalid lockout offset")));
//                     }
//                     Some(slot) => slot,
//                 };
//                 let lockout = Lockout::new_with_confirmation_count(
//                     *slot,
//                     u32::from(lockout_offset.confirmation_count)
//                 );
//                 Some(Ok(lockout))
//             });
//         Ok(TowerSync {
//             root,
//             lockouts: lockouts.collect::<Result<_, _>>()?,
//             hash,
//             timestamp,
//             block_id,
//         })
//     }
// }

// Tests -------------------------------------------------------

// #[cfg(test)]
// mod tests {
//     use ::{
//         super::*,
//         crate::error::VoteError,
//         bincode::serialized_size,
//         core::mem::MaybeUninit,
//         itertools::Itertools,
//         rand::Rng,
//         solana_clock::Clock,
//         solana_instruction::error::InstructionError,
//     };

//     #[test]
//     fn test_vote_serialize() {
//         let mut buffer: Vec<u8> = vec![0; VoteState::size_of()];
//         let mut vote_state = VoteState::default();
//         vote_state.votes.resize(MAX_LOCKOUT_HISTORY, LandedVote::default());
//         vote_state.root_slot = Some(1);
//         let versioned = VoteStateVersions::new_current(vote_state);
//         assert!(VoteState::serialize(&versioned, &mut buffer[0..4]).is_err());
//         VoteState::serialize(&versioned, &mut buffer).unwrap();
//         assert_eq!(VoteState::deserialize(&buffer).unwrap(), versioned.convert_to_current());
//     }

//     #[test]
//     fn test_vote_deserialize_into() {
//         // base case
//         let target_vote_state = VoteState::default();
//         let vote_state_buf = bincode
//             ::serialize(&VoteStateVersions::new_current(target_vote_state.clone()))
//             .unwrap();

//         let mut test_vote_state = VoteState::default();
//         VoteState::deserialize_into(&vote_state_buf, &mut test_vote_state).unwrap();

//         assert_eq!(target_vote_state, test_vote_state);

//         // variant
//         // provide 4x the minimum struct size in bytes to ensure we typically touch every field
//         let struct_bytes_x4 = core::mem::size_of::<VoteState>() * 4;
//         for _ in 0..1000 {
//             let raw_data: Vec<u8> = (0..struct_bytes_x4).map(|_| rand::random::<u8>()).collect();
//             let mut unstructured = Unstructured::new(&raw_data);

//             let target_vote_state_versions = VoteStateVersions::arbitrary(
//                 &mut unstructured
//             ).unwrap();
//             let vote_state_buf = bincode::serialize(&target_vote_state_versions).unwrap();
//             let target_vote_state = target_vote_state_versions.convert_to_current();

//             let mut test_vote_state = VoteState::default();
//             VoteState::deserialize_into(&vote_state_buf, &mut test_vote_state).unwrap();

//             assert_eq!(target_vote_state, test_vote_state);
//         }
//     }

//     #[test]
//     fn test_vote_deserialize_into_error() {
//         let target_vote_state = VoteState::new_rand_for_tests(Pubkey::new_unique(), 42);
//         let mut vote_state_buf = bincode
//             ::serialize(&VoteStateVersions::new_current(target_vote_state.clone()))
//             .unwrap();
//         let len = vote_state_buf.len();
//         vote_state_buf.truncate(len - 1);

//         let mut test_vote_state = VoteState::default();
//         VoteState::deserialize_into(&vote_state_buf, &mut test_vote_state).unwrap_err();
//         assert_eq!(test_vote_state, VoteState::default());
//     }

//     #[test]
//     fn test_vote_deserialize_into_uninit() {
//         // base case
//         let target_vote_state = VoteState::default();
//         let vote_state_buf = bincode
//             ::serialize(&VoteStateVersions::new_current(target_vote_state.clone()))
//             .unwrap();

//         let mut test_vote_state = MaybeUninit::uninit();
//         VoteState::deserialize_into_uninit(&vote_state_buf, &mut test_vote_state).unwrap();
//         let test_vote_state = unsafe { test_vote_state.assume_init() };

//         assert_eq!(target_vote_state, test_vote_state);

//         // variant
//         // provide 4x the minimum struct size in bytes to ensure we typically touch every field
//         let struct_bytes_x4 = core::mem::size_of::<VoteState>() * 4;
//         for _ in 0..1000 {
//             let raw_data: Vec<u8> = (0..struct_bytes_x4).map(|_| rand::random::<u8>()).collect();
//             let mut unstructured = Unstructured::new(&raw_data);

//             let target_vote_state_versions = VoteStateVersions::arbitrary(
//                 &mut unstructured
//             ).unwrap();
//             let vote_state_buf = bincode::serialize(&target_vote_state_versions).unwrap();
//             let target_vote_state = target_vote_state_versions.convert_to_current();

//             let mut test_vote_state = MaybeUninit::uninit();
//             VoteState::deserialize_into_uninit(&vote_state_buf, &mut test_vote_state).unwrap();
//             let test_vote_state = unsafe { test_vote_state.assume_init() };

//             assert_eq!(target_vote_state, test_vote_state);
//         }
//     }

//     #[test]
//     fn test_vote_deserialize_into_uninit_nopanic() {
//         // base case
//         let mut test_vote_state = MaybeUninit::uninit();
//         let e = VoteState::deserialize_into_uninit(&[], &mut test_vote_state).unwrap_err();
//         assert_eq!(e, InstructionError::InvalidAccountData);

//         // variant
//         let serialized_len_x4 = serialized_size(&VoteState::default()).unwrap() * 4;
//         let mut rng = rand::thread_rng();
//         for _ in 0..1000 {
//             let raw_data_length = rng.gen_range(1..serialized_len_x4);
//             let mut raw_data: Vec<u8> = (0..raw_data_length).map(|_| rng.gen::<u8>()).collect();

//             // pure random data will ~never have a valid enum tag, so lets help it out
//             if raw_data_length >= 4 && rng.gen::<bool>() {
//                 let tag = rng.gen::<u8>() % 3;
//                 raw_data[0] = tag;
//                 raw_data[1] = 0;
//                 raw_data[2] = 0;
//                 raw_data[3] = 0;
//             }

//             // it is extremely improbable, though theoretically possible, for random bytes to be syntactically valid
//             // so we only check that the parser does not panic and that it succeeds or fails exactly in line with bincode
//             let mut test_vote_state = MaybeUninit::uninit();
//             let test_res = VoteState::deserialize_into_uninit(&raw_data, &mut test_vote_state);
//             let bincode_res = bincode
//                 ::deserialize::<VoteStateVersions>(&raw_data)
//                 .map(|versioned| versioned.convert_to_current());

//             if test_res.is_err() {
//                 assert!(bincode_res.is_err());
//             } else {
//                 let test_vote_state = unsafe { test_vote_state.assume_init() };
//                 assert_eq!(test_vote_state, bincode_res.unwrap());
//             }
//         }
//     }

//     #[test]
//     fn test_vote_deserialize_into_uninit_ill_sized() {
//         // provide 4x the minimum struct size in bytes to ensure we typically touch every field
//         let struct_bytes_x4 = core::mem::size_of::<VoteState>() * 4;
//         for _ in 0..1000 {
//             let raw_data: Vec<u8> = (0..struct_bytes_x4).map(|_| rand::random::<u8>()).collect();
//             let mut unstructured = Unstructured::new(&raw_data);

//             let original_vote_state_versions = VoteStateVersions::arbitrary(
//                 &mut unstructured
//             ).unwrap();
//             let original_buf = bincode::serialize(&original_vote_state_versions).unwrap();

//             let mut truncated_buf = original_buf.clone();
//             let mut expanded_buf = original_buf.clone();

//             truncated_buf.resize(original_buf.len() - 8, 0);
//             expanded_buf.resize(original_buf.len() + 8, 0);

//             // truncated fails
//             let mut test_vote_state = MaybeUninit::uninit();
//             let test_res = VoteState::deserialize_into_uninit(&truncated_buf, &mut test_vote_state);
//             let bincode_res = bincode
//                 ::deserialize::<VoteStateVersions>(&truncated_buf)
//                 .map(|versioned| versioned.convert_to_current());

//             assert!(test_res.is_err());
//             assert!(bincode_res.is_err());

//             // expanded succeeds
//             let mut test_vote_state = MaybeUninit::uninit();
//             VoteState::deserialize_into_uninit(&expanded_buf, &mut test_vote_state).unwrap();
//             let bincode_res = bincode
//                 ::deserialize::<VoteStateVersions>(&expanded_buf)
//                 .map(|versioned| versioned.convert_to_current());

//             let test_vote_state = unsafe { test_vote_state.assume_init() };
//             assert_eq!(test_vote_state, bincode_res.unwrap());
//         }
//     }

//     #[test]
//     #[allow(deprecated)]
//     fn test_vote_state_commission_split() {
//         let vote_state = VoteState::default();

//         assert_eq!(vote_state.commission_split(1), (0, 1, false));

//         let mut vote_state = VoteState {
//             commission: u8::MAX,
//             ..VoteState::default()
//         };
//         assert_eq!(vote_state.commission_split(1), (1, 0, false));

//         vote_state.commission = 99;
//         assert_eq!(vote_state.commission_split(10), (9, 0, true));

//         vote_state.commission = 1;
//         assert_eq!(vote_state.commission_split(10), (0, 9, true));

//         vote_state.commission = 50;
//         let (voter_portion, staker_portion, was_split) = vote_state.commission_split(10);

//         assert_eq!((voter_portion, staker_portion, was_split), (5, 5, true));
//     }

//     #[test]
//     fn test_vote_state_epoch_credits() {
//         let mut vote_state = VoteState::default();

//         assert_eq!(vote_state.credits(), 0);
//         assert_eq!(vote_state.epoch_credits().clone(), vec![]);

//         let mut expected = vec![];
//         let mut credits = 0;
//         let epochs = (MAX_EPOCH_CREDITS_HISTORY + 2) as u64;
//         for epoch in 0..epochs {
//             for _j in 0..epoch {
//                 vote_state.increment_credits(epoch, 1);
//                 credits += 1;
//             }
//             expected.push((epoch, credits, credits - epoch));
//         }

//         while expected.len() > MAX_EPOCH_CREDITS_HISTORY {
//             expected.remove(0);
//         }

//         assert_eq!(vote_state.credits(), credits);
//         assert_eq!(vote_state.epoch_credits().clone(), expected);
//     }

//     #[test]
//     fn test_vote_state_epoch0_no_credits() {
//         let mut vote_state = VoteState::default();

//         assert_eq!(vote_state.epoch_credits().len(), 0);
//         vote_state.increment_credits(1, 1);
//         assert_eq!(vote_state.epoch_credits().len(), 1);

//         vote_state.increment_credits(2, 1);
//         assert_eq!(vote_state.epoch_credits().len(), 2);
//     }

//     #[test]
//     fn test_vote_state_increment_credits() {
//         let mut vote_state = VoteState::default();

//         let credits = (MAX_EPOCH_CREDITS_HISTORY + 2) as u64;
//         for i in 0..credits {
//             vote_state.increment_credits(i, 1);
//         }
//         assert_eq!(vote_state.credits(), credits);
//         assert!(vote_state.epoch_credits().len() <= MAX_EPOCH_CREDITS_HISTORY);
//     }

//     #[test]
//     fn test_vote_process_timestamp() {
//         let (slot, timestamp) = (15, 1_575_412_285);
//         let mut vote_state = VoteState {
//             last_timestamp: BlockTimestamp { slot, timestamp },
//             ..VoteState::default()
//         };

//         assert_eq!(
//             vote_state.process_timestamp(slot - 1, timestamp + 1),
//             Err(VoteError::TimestampTooOld)
//         );
//         assert_eq!(vote_state.last_timestamp, BlockTimestamp { slot, timestamp });
//         assert_eq!(
//             vote_state.process_timestamp(slot + 1, timestamp - 1),
//             Err(VoteError::TimestampTooOld)
//         );
//         assert_eq!(
//             vote_state.process_timestamp(slot, timestamp + 1),
//             Err(VoteError::TimestampTooOld)
//         );
//         assert_eq!(vote_state.process_timestamp(slot, timestamp), Ok(()));
//         assert_eq!(vote_state.last_timestamp, BlockTimestamp { slot, timestamp });
//         assert_eq!(vote_state.process_timestamp(slot + 1, timestamp), Ok(()));
//         assert_eq!(vote_state.last_timestamp, BlockTimestamp {
//             slot: slot + 1,
//             timestamp,
//         });
//         assert_eq!(vote_state.process_timestamp(slot + 2, timestamp + 1), Ok(()));
//         assert_eq!(vote_state.last_timestamp, BlockTimestamp {
//             slot: slot + 2,
//             timestamp: timestamp + 1,
//         });

//         // Test initial vote
//         vote_state.last_timestamp = BlockTimestamp::default();
//         assert_eq!(vote_state.process_timestamp(0, timestamp), Ok(()));
//     }

//     #[test]
//     fn test_get_and_update_authorized_voter() {
//         let original_voter = Pubkey::new_unique();
//         let mut vote_state = VoteState::new(
//             &(VoteInit {
//                 node_pubkey: original_voter,
//                 authorized_voter: original_voter,
//                 authorized_withdrawer: original_voter,
//                 commission: 0,
//             }),
//             &Clock::default()
//         );

//         assert_eq!(vote_state.authorized_voters.len(), 1);
//         assert_eq!(*vote_state.authorized_voters.first().unwrap().1, original_voter);

//         // If no new authorized voter was set, the same authorized voter
//         // is locked into the next epoch
//         assert_eq!(vote_state.get_and_update_authorized_voter(1).unwrap(), original_voter);

//         // Try to get the authorized voter for epoch 5, implies
//         // the authorized voter for epochs 1-4 were unchanged
//         assert_eq!(vote_state.get_and_update_authorized_voter(5).unwrap(), original_voter);

//         // Authorized voter for expired epoch 0..5 should have been
//         // purged and no longer queryable
//         assert_eq!(vote_state.authorized_voters.len(), 1);
//         for i in 0..5 {
//             assert!(vote_state.authorized_voters.get_authorized_voter(i).is_none());
//         }

//         // Set an authorized voter change at slot 7
//         let new_authorized_voter = Pubkey::new_unique();
//         vote_state.set_new_authorized_voter(&new_authorized_voter, 5, 7, |_| Ok(())).unwrap();

//         // Try to get the authorized voter for epoch 6, unchanged
//         assert_eq!(vote_state.get_and_update_authorized_voter(6).unwrap(), original_voter);

//         // Try to get the authorized voter for epoch 7 and onwards, should
//         // be the new authorized voter
//         for i in 7..10 {
//             assert_eq!(
//                 vote_state.get_and_update_authorized_voter(i).unwrap(),
//                 new_authorized_voter
//             );
//         }
//         assert_eq!(vote_state.authorized_voters.len(), 1);
//     }

//     #[test]
//     fn test_set_new_authorized_voter() {
//         let original_voter = Pubkey::new_unique();
//         let epoch_offset = 15;
//         let mut vote_state = VoteState::new(
//             &(VoteInit {
//                 node_pubkey: original_voter,
//                 authorized_voter: original_voter,
//                 authorized_withdrawer: original_voter,
//                 commission: 0,
//             }),
//             &Clock::default()
//         );

//         assert!(vote_state.prior_voters.last().is_none());

//         let new_voter = Pubkey::new_unique();
//         // Set a new authorized voter
//         vote_state.set_new_authorized_voter(&new_voter, 0, epoch_offset, |_| Ok(())).unwrap();

//         assert_eq!(vote_state.prior_voters.idx, 0);
//         assert_eq!(vote_state.prior_voters.last(), Some(&(original_voter, 0, epoch_offset)));

//         // Trying to set authorized voter for same epoch again should fail
//         assert_eq!(
//             vote_state.set_new_authorized_voter(&new_voter, 0, epoch_offset, |_| Ok(())),
//             Err(VoteError::TooSoonToReauthorize.into())
//         );

//         // Setting the same authorized voter again should succeed
//         vote_state.set_new_authorized_voter(&new_voter, 2, 2 + epoch_offset, |_| Ok(())).unwrap();

//         // Set a third and fourth authorized voter
//         let new_voter2 = Pubkey::new_unique();
//         vote_state.set_new_authorized_voter(&new_voter2, 3, 3 + epoch_offset, |_| Ok(())).unwrap();
//         assert_eq!(vote_state.prior_voters.idx, 1);
//         assert_eq!(
//             vote_state.prior_voters.last(),
//             Some(&(new_voter, epoch_offset, 3 + epoch_offset))
//         );

//         let new_voter3 = Pubkey::new_unique();
//         vote_state.set_new_authorized_voter(&new_voter3, 6, 6 + epoch_offset, |_| Ok(())).unwrap();
//         assert_eq!(vote_state.prior_voters.idx, 2);
//         assert_eq!(
//             vote_state.prior_voters.last(),
//             Some(&(new_voter2, 3 + epoch_offset, 6 + epoch_offset))
//         );

//         // Check can set back to original voter
//         vote_state
//             .set_new_authorized_voter(&original_voter, 9, 9 + epoch_offset, |_| Ok(()))
//             .unwrap();

//         // Run with these voters for a while, check the ranges of authorized
//         // voters is correct
//         for i in 9..epoch_offset {
//             assert_eq!(vote_state.get_and_update_authorized_voter(i).unwrap(), original_voter);
//         }
//         for i in epoch_offset..3 + epoch_offset {
//             assert_eq!(vote_state.get_and_update_authorized_voter(i).unwrap(), new_voter);
//         }
//         for i in 3 + epoch_offset..6 + epoch_offset {
//             assert_eq!(vote_state.get_and_update_authorized_voter(i).unwrap(), new_voter2);
//         }
//         for i in 6 + epoch_offset..9 + epoch_offset {
//             assert_eq!(vote_state.get_and_update_authorized_voter(i).unwrap(), new_voter3);
//         }
//         for i in 9 + epoch_offset..=10 + epoch_offset {
//             assert_eq!(vote_state.get_and_update_authorized_voter(i).unwrap(), original_voter);
//         }
//     }

//     #[test]
//     fn test_authorized_voter_is_locked_within_epoch() {
//         let original_voter = Pubkey::new_unique();
//         let mut vote_state = VoteState::new(
//             &(VoteInit {
//                 node_pubkey: original_voter,
//                 authorized_voter: original_voter,
//                 authorized_withdrawer: original_voter,
//                 commission: 0,
//             }),
//             &Clock::default()
//         );

//         // Test that it's not possible to set a new authorized
//         // voter within the same epoch, even if none has been
//         // explicitly set before
//         let new_voter = Pubkey::new_unique();
//         assert_eq!(
//             vote_state.set_new_authorized_voter(&new_voter, 1, 1, |_| Ok(())),
//             Err(VoteError::TooSoonToReauthorize.into())
//         );

//         assert_eq!(vote_state.get_authorized_voter(1), Some(original_voter));

//         // Set a new authorized voter for a future epoch
//         assert_eq!(
//             vote_state.set_new_authorized_voter(&new_voter, 1, 2, |_| Ok(())),
//             Ok(())
//         );

//         // Test that it's not possible to set a new authorized
//         // voter within the same epoch, even if none has been
//         // explicitly set before
//         assert_eq!(
//             vote_state.set_new_authorized_voter(&original_voter, 3, 3, |_| Ok(())),
//             Err(VoteError::TooSoonToReauthorize.into())
//         );

//         assert_eq!(vote_state.get_authorized_voter(3), Some(new_voter));
//     }

//     #[test]
//     fn test_vote_state_size_of() {
//         let vote_state = VoteState::get_max_sized_vote_state();
//         let vote_state = VoteStateVersions::new_current(vote_state);
//         let size = serialized_size(&vote_state).unwrap();
//         assert_eq!(VoteState::size_of() as u64, size);
//     }

//     #[test]
//     fn test_vote_state_max_size() {
//         let mut max_sized_data = vec![0; VoteState::size_of()];
//         let vote_state = VoteState::get_max_sized_vote_state();
//         let (start_leader_schedule_epoch, _) = vote_state.authorized_voters.last().unwrap();
//         let start_current_epoch =
//             start_leader_schedule_epoch - MAX_LEADER_SCHEDULE_EPOCH_OFFSET + 1;

//         let mut vote_state = Some(vote_state);
//         for i in start_current_epoch..start_current_epoch + 2 * MAX_LEADER_SCHEDULE_EPOCH_OFFSET {
//             vote_state
//                 .as_mut()
//                 .map(|vote_state| {
//                     vote_state.set_new_authorized_voter(
//                         &Pubkey::new_unique(),
//                         i,
//                         i + MAX_LEADER_SCHEDULE_EPOCH_OFFSET,
//                         |_| Ok(())
//                     )
//                 });

//             let versioned = VoteStateVersions::new_current(vote_state.take().unwrap());
//             VoteState::serialize(&versioned, &mut max_sized_data).unwrap();
//             vote_state = Some(versioned.convert_to_current());
//         }
//     }

//     #[test]
//     fn test_default_vote_state_is_uninitialized() {
//         // The default `VoteState` is stored to de-initialize a zero-balance vote account,
//         // so must remain such that `VoteStateVersions::is_uninitialized()` returns true
//         // when called on a `VoteStateVersions` that stores it
//         assert!(VoteStateVersions::new_current(VoteState::default()).is_uninitialized());
//     }

//     #[test]
//     fn test_is_correct_size_and_initialized() {
//         // Check all zeroes
//         let mut vote_account_data = vec![0; VoteStateVersions::vote_state_size_of(true)];
//         assert!(!VoteStateVersions::is_correct_size_and_initialized(&vote_account_data));

//         // Check default VoteState
//         let default_account_state = VoteStateVersions::new_current(VoteState::default());
//         VoteState::serialize(&default_account_state, &mut vote_account_data).unwrap();
//         assert!(!VoteStateVersions::is_correct_size_and_initialized(&vote_account_data));

//         // Check non-zero data shorter than offset index used
//         let short_data = vec![1; DEFAULT_PRIOR_VOTERS_OFFSET];
//         assert!(!VoteStateVersions::is_correct_size_and_initialized(&short_data));

//         // Check non-zero large account
//         let mut large_vote_data = vec![1; 2 * VoteStateVersions::vote_state_size_of(true)];
//         let default_account_state = VoteStateVersions::new_current(VoteState::default());
//         VoteState::serialize(&default_account_state, &mut large_vote_data).unwrap();
//         assert!(!VoteStateVersions::is_correct_size_and_initialized(&vote_account_data));

//         // Check populated VoteState
//         let vote_state = VoteState::new(
//             &(VoteInit {
//                 node_pubkey: Pubkey::new_unique(),
//                 authorized_voter: Pubkey::new_unique(),
//                 authorized_withdrawer: Pubkey::new_unique(),
//                 commission: 0,
//             }),
//             &Clock::default()
//         );
//         let account_state = VoteStateVersions::new_current(vote_state.clone());
//         VoteState::serialize(&account_state, &mut vote_account_data).unwrap();
//         assert!(VoteStateVersions::is_correct_size_and_initialized(&vote_account_data));

//         // Check old VoteState that hasn't been upgraded to newest version yet
//         let old_vote_state = VoteState1_14_11::from(vote_state);
//         let account_state = VoteStateVersions::V1_14_11(Box::new(old_vote_state));
//         let mut vote_account_data = vec![0; VoteStateVersions::vote_state_size_of(false)];
//         VoteState::serialize(&account_state, &mut vote_account_data).unwrap();
//         assert!(VoteStateVersions::is_correct_size_and_initialized(&vote_account_data));
//     }

//     #[test]
//     fn test_minimum_balance() {
//         let rent = solana_rent::Rent::default();
//         let minimum_balance = rent.minimum_balance(VoteState::size_of());
//         // golden, may need updating when vote_state grows
//         assert!((minimum_balance as f64) / (10f64).powf(9.0) < 0.04)
//     }

//     #[test]
//     fn test_serde_compact_vote_state_update() {
//         let mut rng = rand::thread_rng();
//         for _ in 0..5000 {
//             run_serde_compact_vote_state_update(&mut rng);
//         }
//     }

//     fn run_serde_compact_vote_state_update<R: Rng>(rng: &mut R) {
//         let lockouts: VecDeque<_> = core::iter
//             ::repeat_with(|| {
//                 let slot = (149_303_885_u64).saturating_add(rng.gen_range(0..10_000));
//                 let confirmation_count = rng.gen_range(0..33);
//                 Lockout::new_with_confirmation_count(slot, confirmation_count)
//             })
//             .take(32)
//             .sorted_by_key(|lockout| lockout.slot())
//             .collect();
//         let root = rng.gen_ratio(1, 2).then(|| {
//             lockouts[0]
//                 .slot()
//                 .checked_sub(rng.gen_range(0..1_000))
//                 .expect("All slots should be greater than 1_000")
//         });
//         let timestamp = rng.gen_ratio(1, 2).then(|| rng.gen());
//         let hash = Hash::from(rng.gen::<[u8; 32]>());
//         let vote_state_update = VoteStateUpdate {
//             lockouts,
//             root,
//             hash,
//             timestamp,
//         };
//         #[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
//         enum VoteInstruction {
//             #[serde(with = "serde_compact_vote_state_update")] UpdateVoteState(VoteStateUpdate),
//             UpdateVoteStateSwitch(
//                 #[serde(with = "serde_compact_vote_state_update")] VoteStateUpdate,
//                 Hash,
//             ),
//         }
//         let vote = VoteInstruction::UpdateVoteState(vote_state_update.clone());
//         let bytes = bincode::serialize(&vote).unwrap();
//         assert_eq!(vote, bincode::deserialize(&bytes).unwrap());
//         let hash = Hash::from(rng.gen::<[u8; 32]>());
//         let vote = VoteInstruction::UpdateVoteStateSwitch(vote_state_update, hash);
//         let bytes = bincode::serialize(&vote).unwrap();
//         assert_eq!(vote, bincode::deserialize(&bytes).unwrap());
//     }

//     #[test]
//     fn test_circbuf_oob() {
//         // Craft an invalid CircBuf with out-of-bounds index
//         let data: &[u8] = &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00];
//         let circ_buf: CircBuf<()> = bincode::deserialize(data).unwrap();
//         assert_eq!(circ_buf.last(), None);
//     }
// }
