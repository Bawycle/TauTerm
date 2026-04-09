// SPDX-License-Identifier: MPL-2.0

use crate::vt::processor::VtProcessor;

/// Extract the first CSI parameter (0 if absent).
pub(super) fn param0(params: &vte::Params) -> u16 {
    params
        .iter()
        .next()
        .and_then(|p| p.first().copied())
        .unwrap_or(0)
}

/// Extract the second CSI parameter (0 if absent).
pub(super) fn param1(params: &vte::Params) -> u16 {
    params
        .iter()
        .nth(1)
        .and_then(|p| p.first().copied())
        .unwrap_or(0)
}

/// Adjusts `col` left by one if the cell at `(row, col)` is a phantom cell
/// (the trailing width-0 slot of a wide character), so the cursor always
/// rests on the base cell (FS-VT-058).
///
/// Must be called after every absolute column-setting operation:
/// CUP/HVP, CHA, HPA, VPA, DECRC (ESC 8 and CSI u).
pub(super) fn normalize_phantom_col(p: &VtProcessor, row: u16, col: u16) -> u16 {
    if col > 0
        && p.active_buf_ref()
            .get(row, col)
            .is_some_and(|c| c.is_phantom())
    {
        col - 1
    } else {
        col
    }
}
