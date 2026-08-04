//! L2CAP channel management over ACL: signaling, fixed and dynamic channels, and
//! the single-session multiplexing RFCOMM and SDP need. No ERTM; basic mode only.

use crate::error::BluetoothError;

/// Fixed CIDs defined by the Core specification.
pub const CID_SIGNALING: u16 = 0x0001;

/// PSMs Tandem connects to on the audio gateway.
pub const PSM_SDP: u16 = 0x0001;
pub const PSM_RFCOMM: u16 = 0x0003;

/// Dynamic CIDs start here; the host allocates from this range.
pub const FIRST_DYNAMIC_CID: u16 = 0x0040;

/// Allocates locally-scoped channel identifiers for outbound channels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CidAllocator {
    next: u16,
}

impl Default for CidAllocator {
    fn default() -> Self {
        Self {
            next: FIRST_DYNAMIC_CID,
        }
    }
}

impl CidAllocator {
    pub fn allocate(&mut self) -> Result<u16, BluetoothError> {
        if self.next == u16::MAX {
            return Err(BluetoothError::Rfcomm("no free L2CAP CIDs".into()));
        }
        let cid = self.next;
        self.next += 1;
        Ok(cid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_cids_start_above_the_fixed_range() {
        let mut alloc = CidAllocator::default();
        let first = alloc.allocate().unwrap();
        assert_eq!(first, FIRST_DYNAMIC_CID);
        assert!(first > CID_SIGNALING);
        assert_eq!(alloc.allocate().unwrap(), FIRST_DYNAMIC_CID + 1);
    }
}
