// SPDX-License-Identifier: MPL-2.0

//! Integration test that generates TypeScript bindings from the Rust IPC types.
//!
//! Running this test produces `src/lib/ipc/bindings.ts` at the project root.
//! The generated file is committed to the repository (not gitignored) so that
//! PR reviews surface type changes. CI verifies that the committed file is
//! up-to-date via `git diff --exit-code src/lib/ipc/bindings.ts`.

#[test]
fn export_bindings() {
    tau_term_lib::ipc::make_builder()
        .export(
            specta_typescript::Typescript::default(),
            "../src/lib/ipc/bindings.ts",
        )
        .expect("Failed to export TypeScript bindings");
}
