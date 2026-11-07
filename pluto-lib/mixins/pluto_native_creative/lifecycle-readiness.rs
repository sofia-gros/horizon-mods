// Observe the official lifecycle owner. No parallel state or transitions.
use super::{NativeAuthority, NativeLifecycle, NativeWorldEpoch, OpenState};

/// An insertion-capable server open, not merely a still-live handle epoch.
/// A frozen/paused Ready world remains Ready; callers retain timing policy.
#[allow(dead_code)]
pub(in crate::game) fn ready_server_epoch(lifecycle: &NativeLifecycle) -> Option<NativeWorldEpoch> {
    let attempt = lifecycle.attempt.as_ref()?;
    (attempt.session == OpenState::Ready
        && matches!(attempt.authority,
            NativeAuthority::IntegratedServer | NativeAuthority::DedicatedServer))
        .then(|| NativeWorldEpoch::new(attempt.epoch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_lifecycle_rejects_opening_closing_failed_and_closed() {
        for authority in [NativeAuthority::IntegratedServer, NativeAuthority::DedicatedServer] {
            let mut lifecycle = NativeLifecycle::default();
            assert_eq!(ready_server_epoch(&lifecycle), None);
            lifecycle.begin_open(authority);
            assert!(lifecycle.epoch().is_some());
            assert_eq!(ready_server_epoch(&lifecycle), None);
            let ready = lifecycle.world_open_ready().unwrap();
            assert_eq!(ready_server_epoch(&lifecycle), Some(ready.epoch));
            lifecycle.begin_close();
            assert_eq!(lifecycle.epoch(), Some(ready.epoch));
            assert_eq!(ready_server_epoch(&lifecycle), None);
            lifecycle.world_close_final().unwrap();
            assert_eq!(ready_server_epoch(&lifecycle), None);
            lifecycle.begin_open(authority);
            lifecycle.world_open_failed().unwrap();
            assert_eq!(ready_server_epoch(&lifecycle), None);
            lifecycle.begin_open(authority);
            lifecycle.world_open_ready().unwrap();
            lifecycle.world_close_failed().unwrap();
            assert_eq!(ready_server_epoch(&lifecycle), None);
        }
    }

    #[test]
    fn real_lifecycle_rejects_mirror_and_remote_roles_even_when_ready() {
        for authority in [NativeAuthority::LocalAuthoritative, NativeAuthority::RemoteClient] {
            let mut lifecycle = NativeLifecycle::default();
            lifecycle.begin_open(authority);
            lifecycle.world_open_ready().unwrap();
            assert!(lifecycle.epoch().is_some());
            assert_eq!(ready_server_epoch(&lifecycle), None);
        }
    }
}
