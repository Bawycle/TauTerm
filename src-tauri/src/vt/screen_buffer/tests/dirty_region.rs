// SPDX-License-Identifier: MPL-2.0

use super::super::dirty_region::DirtyRegion;

#[test]
fn cursor_moved_makes_non_empty() {
    let mut d = DirtyRegion::default();
    d.mark_cursor_moved();
    assert!(
        !d.is_empty(),
        "cursor_moved must make DirtyRegion non-empty"
    );
}

#[test]
fn merge_propagates_cursor_moved() {
    let mut a = DirtyRegion::default();
    let mut b = DirtyRegion::default();
    b.mark_cursor_moved();
    a.merge(&b);
    assert!(
        a.cursor_moved,
        "merge must propagate cursor_moved from source"
    );
    assert!(!a.is_empty());
}

#[test]
fn dirty_region_mark_full_redraw_overrides_rows() {
    let mut region = DirtyRegion::default();
    region.mark_row(3);
    region.mark_full_redraw();
    assert!(region.is_full_redraw);
    assert!(region.rows.is_empty());
}

#[test]
fn dirty_region_merge_propagates_full_redraw() {
    let mut a = DirtyRegion::default();
    a.mark_row(1);
    let mut b = DirtyRegion::default();
    b.mark_full_redraw();
    a.merge(&b);
    assert!(a.is_full_redraw);
}
