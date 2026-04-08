// SPDX-License-Identifier: MPL-2.0

use super::super::dirty_rows::DirtyRows;

#[test]
fn dirty_rows_set_and_contains() {
    let mut dr = DirtyRows::default();
    assert!(dr.is_empty());
    dr.set(0);
    dr.set(63);
    dr.set(64);
    dr.set(127);
    dr.set(128);
    dr.set(191);
    dr.set(192);
    dr.set(255);
    assert!(dr.contains(0));
    assert!(dr.contains(63));
    assert!(dr.contains(64));
    assert!(dr.contains(127));
    assert!(dr.contains(128));
    assert!(dr.contains(191));
    assert!(dr.contains(192));
    assert!(dr.contains(255));
    assert!(!dr.contains(1));
    assert!(!dr.is_empty());
}

#[test]
fn dirty_rows_out_of_range_silently_ignored() {
    let mut dr = DirtyRows::default();
    dr.set(256);
    dr.set(1000);
    assert!(dr.is_empty(), "out-of-range rows must not set any bit");
}

#[test]
fn dirty_rows_iter_yields_sorted_set_bits() {
    let mut dr = DirtyRows::default();
    let expected: Vec<u16> = vec![0, 5, 63, 64, 127];
    for &row in &expected {
        dr.set(row);
    }
    let collected: Vec<u16> = dr.iter().collect();
    assert_eq!(collected, expected);
}

#[test]
fn dirty_rows_merge_from_combines_bits() {
    let mut a = DirtyRows::default();
    let mut b = DirtyRows::default();
    a.set(0);
    a.set(128);
    b.set(63);
    b.set(191);
    a.merge_from(&b);
    assert!(a.contains(0));
    assert!(a.contains(63));
    assert!(a.contains(128));
    assert!(a.contains(191));
    assert!(!a.contains(1));
}

#[test]
fn dirty_rows_clear_resets_all_bits() {
    let mut dr = DirtyRows::default();
    dr.set(0);
    dr.set(255);
    dr.clear();
    assert!(dr.is_empty());
}
