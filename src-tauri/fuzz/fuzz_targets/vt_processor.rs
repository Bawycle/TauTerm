// SPDX-License-Identifier: MPL-2.0
//
// SEC-PTY-008 — Fuzz VtProcessor against arbitrary byte sequences.
//
// Rationale: the VT parser processes untrusted data from a PTY or SSH channel.
// Any panic or out-of-bounds access on attacker-controlled input is a security
// defect.  This target feeds arbitrary bytes through `VtProcessor::process` so
// that libFuzzer can discover panics, assertion failures, and integer overflows.
//
// To run (requires nightly toolchain):
//
//   cd src-tauri
//   rustup override set nightly   # or: cargo +nightly fuzz build vt_processor
//   cargo fuzz build vt_processor
//   cargo fuzz run vt_processor
//
// CI integration: add `cargo +nightly fuzz run vt_processor -- -max_total_time=60`
// to the security workflow.  See docs/test-protocols/security-pty-ipc-ssh-credentials-csp-osc52.md §SEC-PTY-008.

#![no_main]
use libfuzzer_sys::fuzz_target;
use tau_term_lib::vt::VtProcessor;

fuzz_target!(|data: &[u8]| {
    // 80×24 terminal, 10 000 scrollback lines — matches typical E2E defaults.
    let mut proc = VtProcessor::new(80, 24, 10_000, 0, false);
    proc.process(data);
});
