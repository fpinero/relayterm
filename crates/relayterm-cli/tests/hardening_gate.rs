//! Candidate-level M11 regression gate.
//!
//! The retained native TUI and worktree scenarios are compiled into this
//! target so the documented M11 command cannot pass while either product
//! boundary is broken. Their original targets remain available for focused
//! diagnosis and milestone regression runs.

use relayterm_protocol::{
    COLLECTION_PAGE_BYTES, CONNECTION_BUFFER_LIMIT, JSON_FRAME_LIMIT, MAX_PAGE_SIZE,
    SNAPSHOT_STAGING_LIMIT, TERMINAL_DATA_LIMIT,
};
use std::process::Command;

#[path = "tui_gate.rs"]
mod tui;
#[path = "worktree_gate.rs"]
mod worktrees;

#[test]
fn public_resource_limits_are_finite_and_internally_ordered() {
    assert_eq!(MAX_PAGE_SIZE, 200);
    assert_eq!(TERMINAL_DATA_LIMIT, 16 * 1024);
    assert_eq!(JSON_FRAME_LIMIT, 8 * 1024 * 1024);
    assert_eq!(CONNECTION_BUFFER_LIMIT, 16 * 1024 * 1024);
    assert_eq!(COLLECTION_PAGE_BYTES, 6 * 1024 * 1024);
    assert_eq!(SNAPSHOT_STAGING_LIMIT, 64 * 1024 * 1024);

    let help = Command::new(env!("CARGO_BIN_EXE_rt"))
        .arg("backup")
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    assert!(help.contains("create"));
    assert!(help.contains("restore"));
}
