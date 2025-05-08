pub(crate) mod merge;
pub(crate) use merge::*;
use pinocchio::program_error::ProgramError;

pub(crate) fn checked_add(a: u64, b: u64) -> Result<u64, ProgramError> {
    a.checked_add(b).ok_or(ProgramError::InsufficientFunds)
}
