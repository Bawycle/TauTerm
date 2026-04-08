// SPDX-License-Identifier: MPL-2.0

use super::super::api::SearchQuery;

// -----------------------------------------------------------------------
// TEST-VT-022 — Search on alternate screen returns zero results
//
// The alternate screen has no scrollback (FS-SB-004). `VtProcessor::search`
// searches `normal.scrollback_iter()` — when the alternate screen is active
// that iterator still reflects the *normal* scrollback. The spec requires
// that a search issued while the alternate screen is active returns 0 results
// because the alternate buffer does not participate in scrollback search.
// -----------------------------------------------------------------------

/// TEST-VT-022: search while alternate screen is active returns 0 results.
///
/// We inject content through the normal screen (so it ends up in scrollback),
/// then switch to the alternate screen and verify search returns nothing.
#[test]
fn test_vt_022_search_on_alternate_screen_returns_zero_results() {
    use crate::vt::processor::VtProcessor;

    let mut vt = VtProcessor::new(80, 24, 1000);

    // Write some text to the normal screen and scroll it into the scrollback
    // by filling more lines than the visible rows (25 > 24).
    let mut data = b"needle\r\n".to_vec();
    for _ in 0..24 {
        data.extend_from_slice(b"other\r\n");
    }
    vt.process(&data);

    let query = SearchQuery {
        text: "needle".to_string(),
        case_sensitive: true,
        regex: false,
    };
    let normal_results = vt.search(&query);
    assert!(
        !normal_results.is_empty(),
        "TEST-VT-022 prerequisite: 'needle' must be found in normal scrollback"
    );

    // Switch to alternate screen (CSI ? 1049 h).
    vt.process(b"\x1b[?1049h");

    // Search while alternate screen is active must return 0 results.
    let alt_results = vt.search(&query);
    assert_eq!(
        alt_results.len(),
        0,
        "TEST-VT-022: search while alternate screen is active must return 0 results (FS-SB-004)"
    );

    // Restore normal screen (no state leak between tests).
    vt.process(b"\x1b[?1049l");
}
