//! LAN peer discovery for the serverless mesh (mDNS via `mdns-sd`).
//!
//! Each mesh node advertises a `_dipralix._tcp.local.` service carrying its
//! room and user in TXT records, and browses the same service type to find
//! peers. Discovery is room-scoped: a node only dials peers whose advertised
//! `room` matches its own. No central server, STUN, or TURN is involved —
//! this is the "two or three devs on the same network" path from §7.
//!
//! mDNS only reaches the local link (it is multicast to `224.0.0.251`), which
//! is exactly the trust boundary we want for the zero-config mesh: peers must
//! already share the LAN *and* the room secret (see [`super::crypto`]).

use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tracing::{debug, warn};
use uuid::Uuid;

use super::error::{Result, SyncError};

/// The mDNS service type every dipralix mesh node advertises and browses.
pub const SERVICE_TYPE: &str = "_dipralix._tcp.local.";

/// TXT key holding the room name.
const TXT_ROOM: &str = "room";
/// TXT key holding the advertising user.
const TXT_USER: &str = "user";

/// A running mDNS presence: advertises this node and can browse for peers.
///
/// Dropping (or calling [`Discovery::shutdown`]) unregisters the service and
/// stops the daemon so the node disappears from the network promptly.
pub struct Discovery {
    daemon: ServiceDaemon,
    /// Our own service fullname, so browse results can skip ourselves.
    fullname: String,
    room: String,
}

impl Discovery {
    /// Start the mDNS daemon and advertise this node on `port`.
    ///
    /// `instance_hint` (usually the user) is combined with a short random
    /// suffix to form a unique mDNS instance name, so two people called
    /// "alice" on the same LAN do not collide.
    ///
    /// # Errors
    /// Returns [`SyncError::Transport`] if the daemon cannot start or the
    /// service cannot be registered (e.g. no usable network interface).
    pub fn advertise(room: &str, port: u16, instance_hint: &str) -> Result<Self> {
        let daemon =
            ServiceDaemon::new().map_err(|e| SyncError::Transport(format!("mdns daemon: {e}")))?;

        let suffix = Uuid::new_v4().simple().to_string();
        let instance = format!("dipralix-{instance_hint}-{}", &suffix[..8]);
        let host = format!("{instance}.local.");
        let props: &[(&str, &str)] = &[(TXT_ROOM, room), (TXT_USER, instance_hint)];

        let info = ServiceInfo::new(SERVICE_TYPE, &instance, &host, "", port, props)
            .map_err(|e| SyncError::Transport(format!("mdns service info: {e}")))?
            .enable_addr_auto();
        let fullname = info.get_fullname().to_string();

        daemon
            .register(info)
            .map_err(|e| SyncError::Transport(format!("mdns register: {e}")))?;

        debug!(%fullname, %room, port, "advertising mesh node over mDNS");
        Ok(Self {
            daemon,
            fullname,
            room: room.to_string(),
        })
    }

    /// Browse for peers in the same room for up to `timeout`, returning their
    /// socket addresses (deduplicated, excluding ourselves).
    ///
    /// This resolves whatever is currently visible on the link within the
    /// window; callers typically poll it on an interval to pick up peers that
    /// join later.
    ///
    /// # Errors
    /// Returns [`SyncError::Transport`] if the browse cannot be started.
    pub async fn browse(&self, timeout: Duration) -> Result<Vec<SocketAddr>> {
        let receiver = self
            .daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| SyncError::Transport(format!("mdns browse: {e}")))?;

        let mut peers: HashSet<SocketAddr> = HashSet::new();
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, receiver.recv_async()).await {
                Err(_) => break,     // window elapsed
                Ok(Err(_)) => break, // channel closed
                Ok(Ok(event)) => {
                    if let ServiceEvent::ServiceResolved(info) = event {
                        if info.get_fullname() == self.fullname {
                            continue; // ourselves
                        }
                        if info.get_property_val_str(TXT_ROOM) != Some(self.room.as_str()) {
                            continue; // different room
                        }
                        let port = info.get_port();
                        for ip in info.get_addresses() {
                            // Skip link-local/loopback noise; keep routable LAN IPs.
                            if ip.is_loopback() {
                                continue;
                            }
                            peers.insert(SocketAddr::new(*ip, port));
                        }
                    }
                }
            }
        }
        Ok(peers.into_iter().collect())
    }

    /// Unregister and stop the mDNS daemon.
    pub fn shutdown(self) {
        if let Err(e) = self.daemon.unregister(&self.fullname) {
            warn!(error = %e, "mdns unregister failed");
        }
        let _ = self.daemon.shutdown();
    }
}

/// True if `ip` is a usable, non-loopback address we would dial.
#[must_use]
pub fn is_dialable(ip: &IpAddr) -> bool {
    !ip.is_loopback() && !ip.is_unspecified()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_type_is_dipralix_tcp() {
        assert_eq!(SERVICE_TYPE, "_dipralix._tcp.local.");
    }

    #[test]
    fn loopback_is_not_dialable() {
        assert!(!is_dialable(&"127.0.0.1".parse::<IpAddr>().unwrap()));
        assert!(!is_dialable(&"0.0.0.0".parse::<IpAddr>().unwrap()));
        assert!(is_dialable(&"192.168.1.20".parse::<IpAddr>().unwrap()));
    }

    // A full advertise/browse round trip needs a live multicast-capable
    // interface, which CI sandboxes usually lack; that path is exercised
    // by the `mesh_loopback` integration test over TCP instead. Here we
    // only assert the daemon starts and registers without error when a
    // network is present, tolerating the no-network case.
    #[tokio::test]
    async fn advertise_starts_or_reports_no_network() {
        match Discovery::advertise("room-x", 7900, "tester") {
            Ok(d) => d.shutdown(),
            Err(SyncError::Transport(_)) => { /* no usable interface in sandbox */ }
            Err(other) => panic!("unexpected error: {other}"),
        }
    }
}
