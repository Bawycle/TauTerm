// SPDX-License-Identifier: MPL-2.0

use crate::vt::{
    charset::translate_dec_special,
    modes::{Charset, CharsetSlot},
};
use crate::vt::processor::VtProcessor;

pub(super) fn handle_print(p: &mut VtProcessor, c: char) {
    // Apply DEC Special Graphics mapping if active.
    // `u8::try_from` is used per convention (CLAUDE.md) even though the
    // `is_ascii()` guard already guarantees the conversion is lossless.
    let mapped_c = if let Ok(byte) = u8::try_from(c)
        && byte >= 0x60
    {
        let active_charset = match p.modes.charset_slot {
            CharsetSlot::G0 => p.modes.g0,
            CharsetSlot::G1 => p.modes.g1,
        };
        if active_charset == Charset::DecSpecialGraphics {
            translate_dec_special(byte)
        } else {
            c
        }
    } else {
        c
    };
    p.write_char(mapped_c);
}
