// SPDX-License-Identifier: MPL-2.0

// ---------------------------------------------------------------------------
// DirtyRows — compact bitfield for dirty row tracking
// ---------------------------------------------------------------------------

/// Bitfield tracking which rows have pending cell changes.
/// Supports up to 256 rows (4 × u64). Row indices ≥ 256 are silently ignored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirtyRows([u64; 4]);

impl DirtyRows {
    /// Mark row as dirty. Rows ≥ 256 are silently ignored.
    pub fn set(&mut self, row: u16) {
        if row < 256 {
            self.0[row as usize / 64] |= 1u64 << (row % 64);
        }
    }

    pub fn contains(&self, row: u16) -> bool {
        if row >= 256 {
            return false;
        }
        self.0[row as usize / 64] & (1u64 << (row % 64)) != 0
    }

    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|&w| w == 0)
    }

    pub fn clear(&mut self) {
        self.0 = [0; 4];
    }

    /// Merge another `DirtyRows` into this one (bitwise OR).
    pub fn merge_from(&mut self, other: &DirtyRows) {
        for (a, b) in self.0.iter_mut().zip(other.0.iter()) {
            *a |= b;
        }
    }

    /// Iterate over all set row indices in ascending order.
    pub fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        self.0.iter().enumerate().flat_map(|(word_idx, &word)| {
            (0u16..64).filter_map(move |bit| {
                if word & (1u64 << bit) != 0 {
                    Some((word_idx as u16) * 64 + bit)
                } else {
                    None
                }
            })
        })
    }
}
