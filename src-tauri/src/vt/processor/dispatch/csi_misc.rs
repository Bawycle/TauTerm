// SPDX-License-Identifier: MPL-2.0

use crate::vt::processor::VtProcessor;

use super::helpers::normalize_phantom_col;

/// ICH — Insert Character (CSI Ps @)
pub(super) fn ich(p: &mut VtProcessor, n: u16) {
    let row = p.cursor_row();
    let col = p.cursor_col();
    p.active_buf_mut().insert_cells(row, col, n);
}

/// DCH — Delete Character (CSI Ps P)
pub(super) fn dch(p: &mut VtProcessor, n: u16) {
    let row = p.cursor_row();
    let col = p.cursor_col();
    p.active_buf_mut().delete_cells(row, col, n);
}

/// DECSC via CSI s — save cursor
pub(super) fn decsc(p: &mut VtProcessor) {
    let mut saved = p.active_cursor().clone();
    // Capture current SGR attributes, charset slot, DECAWM, and DECOM.
    saved.attrs = p.current_attrs;
    saved.charset_slot = p.modes.charset_slot;
    saved.decawm = p.modes.decawm;
    saved.decom = p.modes.decom;
    if p.alt_active {
        p.saved_alt_cursor = Some(saved);
    } else {
        p.saved_normal_cursor = Some(saved);
    }
}

/// DECRC via CSI u — restore cursor
pub(super) fn decrc(p: &mut VtProcessor) {
    let restored = if p.alt_active {
        p.saved_alt_cursor.clone()
    } else {
        p.saved_normal_cursor.clone()
    };
    if let Some(pos) = restored {
        p.current_attrs = pos.attrs;
        p.modes.charset_slot = pos.charset_slot;
        p.modes.decawm = pos.decawm;
        p.modes.decom = pos.decom;
        *p.active_cursor_mut() = pos;
        // FS-VT-058: normalize cursor away from phantom cells after restore.
        let row = p.cursor_row();
        let col = normalize_phantom_col(p, row, p.cursor_col());
        p.active_cursor_mut().col = col;
        p.wrap_pending = false;
        p.pending_dirty.mark_cursor_moved();
    }
}

/// DSR — Device Status Report (CSI Ps n)
pub(super) fn dsr(p: &mut VtProcessor, param: u16) {
    match param {
        5 => p.pending_responses.push(b"\x1b[0n".to_vec()),
        6 => {
            let row = p.cursor_row();
            // FS-VT-058: report normalized position (defensive, setters already normalize).
            let col = normalize_phantom_col(p, row, p.cursor_col());
            p.pending_responses
                .push(format!("\x1b[{};{}R", row + 1, col + 1).into_bytes());
        }
        _ => {}
    }
}

/// DA — Primary Device Attributes (CSI c / CSI 0c)
pub(super) fn da(p: &mut VtProcessor, param: u16) {
    if param == 0 {
        p.pending_responses.push(b"\x1b[?1;2c".to_vec());
    }
}

/// DECSCUSR — Set Cursor Shape (CSI Ps SP q)
pub(super) fn decscusr(p: &mut VtProcessor, param: u16) {
    // SAFETY: param0 is clamped to [0, 6] — fits in u8 without truncation.
    let shape = param.min(6) as u8;
    if p.cursor_shape != shape {
        p.cursor_shape = shape;
        p.cursor_shape_changed = true;
    }
}
