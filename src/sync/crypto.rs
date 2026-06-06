//! End-to-end crypto for the realtime sync layer (Noise via `snow`).
//!
//! Every mesh frame — and, when enabled, every server-relayed frame — is
//! sealed with a [Noise] session before it touches the wire. We use the
//! `Noise_NNpsk0_25519_ChaChaPoly_BLAKE2s` pattern:
//!
//! - **NN** — neither side has a long-term static key. Both contribute a
//!   fresh ephemeral X25519 key per session, giving forward secrecy.
//! - **psk0** — a 32-byte pre-shared key (the room secret) is mixed in at
//!   the very start of the handshake, so a peer that does not know the room
//!   secret cannot complete the handshake. This is the mutual-authentication
//!   property: knowing the room key *is* the credential.
//! - **ChaChaPoly** — ChaCha20-Poly1305 AEAD for the transport phase.
//! - **BLAKE2s** — hash used inside the Noise state machine.
//!
//! The handshake is exactly two messages (`-> e` / `<- e, ee`), so a TCP
//! peer connection reaches transport mode after one-and-a-half round trips.
//!
//! [Noise]: https://noiseprotocol.org/noise.html

use snow::{Builder, HandshakeState, TransportState};

use super::error::{Result, SyncError};

/// Length of the pre-shared key, in bytes. Fixed at 32 (256 bits) to match
/// Noise's PSK requirement and the output width of BLAKE3.
pub const PSK_LEN: usize = 32;

/// The exact Noise pattern string this module speaks. Both peers must agree
/// on it or the handshake fails immediately.
pub const NOISE_PARAMS: &str = "Noise_NNpsk0_25519_ChaChaPoly_BLAKE2s";

/// Largest Noise message, per the spec. Handshake and transport buffers are
/// sized to this so a single `write_message`/`read_message` never overflows.
const MAX_NOISE_MSG: usize = 65535;

/// A 32-byte pre-shared key, derived from a room secret.
///
/// The room secret is whatever low-entropy string a team agrees on (or the
/// JWT used for the server path); it is stretched to a uniform 32 bytes with
/// BLAKE3 so callers never have to hand-manage key length.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Psk([u8; PSK_LEN]);

impl std::fmt::Debug for Psk {
    /// Never print key material. A leaked PSK in a log defeats the whole
    /// scheme, so `Debug` is deliberately opaque.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Psk(***)")
    }
}

impl Psk {
    /// Derive a PSK from an arbitrary room secret by hashing it with BLAKE3.
    /// Any non-empty secret yields a full-entropy 32-byte key.
    #[must_use]
    pub fn derive(room_secret: &[u8]) -> Self {
        let digest = blake3::hash(room_secret);
        Self(*digest.as_bytes())
    }

    /// Build a PSK from exactly 32 raw bytes.
    ///
    /// # Errors
    /// Returns [`SyncError::Crypto`] if `b` is not [`PSK_LEN`] bytes long.
    pub fn from_bytes(b: &[u8]) -> Result<Self> {
        if b.len() != PSK_LEN {
            return Err(SyncError::Crypto(format!(
                "psk must be {PSK_LEN} bytes, got {}",
                b.len()
            )));
        }
        let mut out = [0u8; PSK_LEN];
        out.copy_from_slice(b);
        Ok(Self(out))
    }

    /// Borrow the raw key bytes (for the Noise builder only).
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// One side of an in-progress Noise handshake.
///
/// Drive it by alternating [`Handshake::write_message`] and
/// [`Handshake::read_message`] according to the role, then call
/// [`Handshake::into_transport`] once [`Handshake::is_finished`] is true.
pub struct Handshake {
    state: HandshakeState,
}

impl Handshake {
    /// Start a handshake as the **initiator** (the peer that dials).
    ///
    /// # Errors
    /// Returns [`SyncError::Crypto`] if the Noise params or PSK are rejected.
    pub fn initiator(psk: &Psk) -> Result<Self> {
        Self::build(psk, true)
    }

    /// Start a handshake as the **responder** (the peer that accepts).
    ///
    /// # Errors
    /// Returns [`SyncError::Crypto`] if the Noise params or PSK are rejected.
    pub fn responder(psk: &Psk) -> Result<Self> {
        Self::build(psk, false)
    }

    fn build(psk: &Psk, initiator: bool) -> Result<Self> {
        let params = NOISE_PARAMS
            .parse()
            .map_err(|e| SyncError::Crypto(format!("bad noise params: {e}")))?;
        let builder = Builder::new(params).psk(0, psk.as_bytes());
        let state = if initiator {
            builder.build_initiator()
        } else {
            builder.build_responder()
        }
        .map_err(|e| SyncError::Crypto(format!("build: {e}")))?;
        Ok(Self { state })
    }

    /// Produce the next outbound handshake message (empty Noise payload).
    ///
    /// # Errors
    /// Returns [`SyncError::Crypto`] if it is not this side's turn to write.
    pub fn write_message(&mut self) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; MAX_NOISE_MSG];
        let len = self
            .state
            .write_message(&[], &mut buf)
            .map_err(|e| SyncError::Crypto(format!("handshake write: {e}")))?;
        buf.truncate(len);
        Ok(buf)
    }

    /// Consume an inbound handshake message.
    ///
    /// # Errors
    /// Returns [`SyncError::Crypto`] if authentication fails — the dominant
    /// cause is a wrong PSK (the peer does not know the room secret).
    pub fn read_message(&mut self, msg: &[u8]) -> Result<()> {
        let mut buf = vec![0u8; MAX_NOISE_MSG];
        self.state
            .read_message(msg, &mut buf)
            .map_err(|e| SyncError::Crypto(format!("handshake read: {e}")))?;
        Ok(())
    }

    /// True once the handshake has completed and a transport can be taken.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.state.is_handshake_finished()
    }

    /// Convert a finished handshake into a [`Transport`] for the data phase.
    ///
    /// # Errors
    /// Returns [`SyncError::Crypto`] if the handshake is not yet complete.
    pub fn into_transport(self) -> Result<Transport> {
        let state = self
            .state
            .into_transport_mode()
            .map_err(|e| SyncError::Crypto(format!("into transport: {e}")))?;
        Ok(Transport { state })
    }
}

/// A live, encrypted Noise transport. Sequence-numbered AEAD under the hood;
/// frames must be processed in order on both ends.
pub struct Transport {
    state: TransportState,
}

impl Transport {
    /// Seal one plaintext frame, returning ciphertext (longer by the AEAD
    /// tag). The result is what goes on the wire.
    ///
    /// # Errors
    /// Returns [`SyncError::Crypto`] if the plaintext exceeds the Noise
    /// message limit or the cipher state is exhausted.
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; plaintext.len() + 16 + 64];
        let len = self
            .state
            .write_message(plaintext, &mut buf)
            .map_err(|e| SyncError::Crypto(format!("encrypt: {e}")))?;
        buf.truncate(len);
        Ok(buf)
    }

    /// Open one ciphertext frame, returning the plaintext.
    ///
    /// # Errors
    /// Returns [`SyncError::Crypto`] if the AEAD tag does not verify — a
    /// tampered, reordered, or wrong-key frame.
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; ciphertext.len() + 64];
        let len = self
            .state
            .read_message(ciphertext, &mut buf)
            .map_err(|e| SyncError::Crypto(format!("decrypt: {e}")))?;
        buf.truncate(len);
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive a full in-memory handshake and return both live transports.
    fn handshake_pair(psk_i: &Psk, psk_r: &Psk) -> Result<(Transport, Transport)> {
        let mut i = Handshake::initiator(psk_i)?;
        let mut r = Handshake::responder(psk_r)?;
        let m1 = i.write_message()?;
        r.read_message(&m1)?;
        let m2 = r.write_message()?;
        i.read_message(&m2)?;
        assert!(i.is_finished() && r.is_finished());
        Ok((i.into_transport()?, r.into_transport()?))
    }

    #[test]
    fn psk_derive_is_deterministic_and_full_width() {
        let a = Psk::derive(b"team-room-secret");
        let b = Psk::derive(b"team-room-secret");
        assert_eq!(a.as_bytes(), b.as_bytes());
        assert_eq!(a.as_bytes().len(), PSK_LEN);
    }

    #[test]
    fn psk_derive_differs_on_secret() {
        let a = Psk::derive(b"room-a");
        let b = Psk::derive(b"room-b");
        assert_ne!(a.as_bytes(), b.as_bytes());
    }

    #[test]
    fn psk_from_bytes_rejects_wrong_length() {
        assert!(Psk::from_bytes(&[1u8; 31]).is_err());
        assert!(Psk::from_bytes(&[1u8; 32]).is_ok());
    }

    #[test]
    fn psk_debug_does_not_leak_key() {
        let psk = Psk::derive(b"secret");
        assert_eq!(format!("{psk:?}"), "Psk(***)");
    }

    #[test]
    fn handshake_completes_with_matching_psk() {
        let psk = Psk::derive(b"shared");
        let pair = handshake_pair(&psk, &psk);
        assert!(pair.is_ok());
    }

    #[test]
    fn handshake_fails_with_mismatched_psk() {
        let alice = Psk::derive(b"room-alpha");
        let mallory = Psk::derive(b"room-omega");
        // The responder cannot authenticate the initiator's first message
        // because the PSK is mixed in before any DH output.
        let mut i = Handshake::initiator(&alice).unwrap();
        let mut r = Handshake::responder(&mallory).unwrap();
        let m1 = i.write_message().unwrap();
        // NNpsk0 mixes the PSK first, so the responder's read of m1 already
        // fails the tag check.
        assert!(r.read_message(&m1).is_err());
    }

    #[test]
    fn transport_round_trips_plaintext() {
        let psk = Psk::derive(b"shared");
        let (mut i, mut r) = handshake_pair(&psk, &psk).unwrap();
        let plaintext = b"{\"type\":\"file_update\"}";
        let ct = i.encrypt(plaintext).unwrap();
        let pt = r.decrypt(&ct).unwrap();
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn ciphertext_is_not_plaintext_on_the_wire() {
        // Phase 4 DoD: a captured frame must be ciphertext.
        let psk = Psk::derive(b"shared");
        let (mut i, _r) = handshake_pair(&psk, &psk).unwrap();
        let plaintext = b"DROP TABLE users";
        let ct = i.encrypt(plaintext).unwrap();
        assert_ne!(&ct[..], &plaintext[..]);
        assert!(!ct.windows(plaintext.len()).any(|w| w == plaintext));
        assert!(ct.len() > plaintext.len(), "AEAD tag must be appended");
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let psk = Psk::derive(b"shared");
        let (mut i, mut r) = handshake_pair(&psk, &psk).unwrap();
        let mut ct = i.encrypt(b"hello").unwrap();
        ct[0] ^= 0xff; // flip a bit
        assert!(r.decrypt(&ct).is_err());
    }
}
