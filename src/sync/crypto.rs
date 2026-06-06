//! Phase 4 end-to-end crypto (Noise / `snow`).
//!
//! This module is currently a stub. The full Noise XX handshake over
//! a per-room pre-shared key (PSK) is scheduled for the post-Phase-4
//! hardening pass; the surrounding transport (WebSocket + P2P mesh)
//! already supports encrypted payloads as opaque bytes, so the
//! remaining work is purely the handshake/cipher-state plumbing.
//!
//! Public surface is deliberately minimal so downstream code that
//! links against `dipralix::sync` keeps compiling while the module
//! is being fleshed out.

/// Re-export of the pre-shared key length used by the planned Noise
/// XX+PSK handshake. Fixed at 32 bytes (256 bits) to match Noise's
/// recommended PSK size and to give us a stable `Copy` type we can
/// hand around in tests.
pub const PSK_LEN: usize = 32;

/// A zeroed pre-shared key. **Never use this in production** — it is
/// only here so unit tests and the CLI `--token-secret` placeholder
/// have a value to bind to until the real key-exchange flow lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Psk([u8; PSK_LEN]);

impl Psk {
    /// Build a PSK from raw bytes, panicking if the slice is the wrong
    /// length. Production callers should derive the PSK from a
    /// 32-byte Blake3 hash of the room secret; this constructor is
    /// for tests and explicit `expect` sites only.
    #[must_use]
    pub fn from_bytes(b: &[u8]) -> Self {
        assert_eq!(b.len(), PSK_LEN, "psk must be exactly {PSK_LEN} bytes");
        let mut out = [0u8; PSK_LEN];
        out.copy_from_slice(b);
        Self(out)
    }

    /// Borrow the raw PSK bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Default for Psk {
    fn default() -> Self {
        Self([0u8; PSK_LEN])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn psk_round_trips_bytes() {
        let bytes = [7u8; PSK_LEN];
        let psk = Psk::from_bytes(&bytes);
        assert_eq!(psk.as_bytes(), &bytes);
    }

    #[test]
    fn psk_default_is_zeroed() {
        let psk = Psk::default();
        assert_eq!(psk.as_bytes(), &[0u8; PSK_LEN]);
    }

    #[test]
    fn psk_from_bytes_wrong_length_panics() {
        let result = std::panic::catch_unwind(|| Psk::from_bytes(&[1u8; 31]));
        assert!(result.is_err(), "expected panic on short PSK");
    }
}
