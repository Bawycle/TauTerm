// SPDX-License-Identifier: MPL-2.0

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
