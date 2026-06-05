//! Dipralix library — exposes the `sync` module so integration tests
//! (and any future external consumers) can link against the same
//! types the `dipralix-cli` binary uses.
//!
//! The other modules of the binary (`agent`, `memory`, `tools`, …)
//! stay private to the binary crate by design — they are implementation
//! details of the CLI and not part of the public surface yet.

pub mod sync;
