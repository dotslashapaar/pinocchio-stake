use pinocchio::pubkey::Pubkey;

use super::{Clock, Epoch, UnixTimestamp};

#[repr(C)]
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub struct Lockup {
    /// UnixTimestamp at which this stake will allow withdrawal, unless the
    ///   transaction is signed by the custodian
    pub unix_timestamp: UnixTimestamp, //i64
    /// epoch height at which this stake will allow withdrawal, unless the
    ///   transaction is signed by the custodian
    pub epoch: Epoch,
    /// custodian signature on a transaction exempts the operation from
    ///  lockup constraints
    pub custodian: Pubkey,
}

impl Lockup {
    #[inline(always)]
    pub fn set_unix_timestamp(&mut self, unix_timestamp: i64) {
        self.unix_timestamp = unix_timestamp.to_le_bytes();
    }

    #[inline(always)]
    pub fn unix_timestamp(&self) -> i64 {
        i64::from_le_bytes(self.unix_timestamp)
    }

    #[inline(always)]
    pub fn set_epoch(&mut self, epoch: u64) {
        self.epoch = epoch.to_le_bytes();
    }

    #[inline(always)]
    pub fn epoch(&self) -> u64 {
        u64::from_le_bytes(self.epoch)
    }

    pub fn is_in_force(&self, clock: &Clock, custodian: Option<&Pubkey>) -> bool {
        if custodian == Some(&self.custodian) {
            return false;
        }

        i64::from_le_bytes(self.unix_timestamp) > i64::from_le_bytes(clock.unix_timestamp)
            || u64::from_le_bytes(self.epoch) > u64::from_le_bytes(clock.epoch)
    }
}
