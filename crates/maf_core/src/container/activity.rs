//! Tracking for container activity. This is used to determine when a container has been idle for
//! long enough to be stopped.

use std::sync::atomic::{AtomicU64, Ordering};

/// A container's activity state. This is used to track when a container was last active, and
/// whether it has been stopped. If a container has been stopped, no further activity will be
/// recorded and a container will be considered idle regardless of its last activity timestamp.
///
/// The top bit marks the container as stopped. The remaining bits store the last activity
/// timestamp in seconds since the Unix epoch.
#[derive(Debug)]
pub struct ActivityState(AtomicU64);

impl ActivityState {
    const STOPPED_BIT: u64 = 1 << 63;
    const ACTIVITY_MASK: u64 = Self::STOPPED_BIT - 1;

    pub fn new(now: u64) -> Self {
        Self(AtomicU64::new(now & Self::ACTIVITY_MASK))
    }

    /// Check if the container has been stopped. If it has, no further activity will be recorded.
    pub fn is_stopped(&self) -> bool {
        // We use `Ordering::Acquire` here to ensure that if we see the stopped bit set, we also see
        // all prior writes to the activity state.
        self.0.load(Ordering::Acquire) & Self::STOPPED_BIT != 0
    }

    /// Mark the container as stopped. This will prevent any further activity from being recorded.
    pub fn stop(&self) {
        // We use `Ordering::AcqRel` here to ensure that all prior writes to the activity state are
        // visible to any thread that sees the stopped bit set and that any subsequent writes to the
        // activity state are not reordered before this write.
        self.0.fetch_or(Self::STOPPED_BIT, Ordering::AcqRel);
    }

    /// Record activity for the container. This will update the last activity timestamp to `now`,
    /// unless the container has been stopped, in which case it will return `false` and not update
    /// the timestamp. Returns `true` if the activity was recorded, or `false` if the container has
    /// been stopped.
    pub fn record_activity(&self, now: u64) -> bool {
        let now = now & Self::ACTIVITY_MASK;

        loop {
            let current = self.0.load(Ordering::Acquire);
            if current & Self::STOPPED_BIT != 0 {
                return false;
            }

            match self.0.compare_exchange(
                current,
                // Preserve whatever the stopped bit is set to, but update the activity timestamp to
                // the new value.
                (current & Self::STOPPED_BIT) | now,
                // On success (whether the CAS succeeded), we want to ensure that all prior writes
                // to the activity state are visible to other threads that see the updated value.
                Ordering::AcqRel,
                // On failure, use `Ordering::Acquire` so that the `observed` value we retry or bail
                // out on reflects all writes visible up to that point (in particular, so we
                // reliably detect a concurrent `stop()`).
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(observed) if observed & Self::STOPPED_BIT != 0 => return false,
                Err(_) => continue,
            }
        }
    }

    /// If the container has been idle for at least `timeout` seconds, atomically mark it as
    /// stopped and return `true`.
    ///
    /// This combines the timeout comparison and the stop transition into one compare-and-swap
    /// loop so a concurrent activity update cannot slip in between the check and the stop.
    pub fn stop_if_inactive(&self, now: u64, timeout: u64) -> bool {
        loop {
            let current = self.0.load(Ordering::Acquire);

            if current & Self::STOPPED_BIT != 0 {
                return false;
            }

            let last_activity = current & Self::ACTIVITY_MASK;
            if now.saturating_sub(last_activity) <= timeout {
                return false;
            }

            match self.0.compare_exchange(
                current,
                current | Self::STOPPED_BIT,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(observed) if observed & Self::STOPPED_BIT != 0 => return false,
                Err(_) => continue,
            }
        }
    }

    /// Get the last activity timestamp in seconds since the Unix epoch.
    pub fn last_activity(&self) -> u64 {
        self.0.load(Ordering::Acquire) & Self::ACTIVITY_MASK
    }
}

#[cfg(test)]
mod tests {
    use super::ActivityState;
    use crate::utils;

    #[test]
    fn activity_state_ignores_updates_after_stop() {
        let activity = ActivityState::new(10);

        assert!(activity.record_activity(12));
        assert_eq!(activity.last_activity(), 12);

        activity.stop();

        assert!(activity.is_stopped());
        assert!(!activity.record_activity(15));
        assert_eq!(activity.last_activity(), 12);
    }

    #[test]
    fn activity_state_can_stop_without_losing_last_activity() {
        let activity = ActivityState::new(utils::now_as_secs());

        let before = activity.last_activity();
        activity.stop();

        assert!(activity.is_stopped());
        assert_eq!(activity.last_activity(), before);
    }

    #[test]
    fn stop_if_inactive_only_stops_after_timeout() {
        let activity = ActivityState::new(100);

        assert!(!activity.stop_if_inactive(120, 30));
        assert!(!activity.is_stopped());

        assert!(activity.stop_if_inactive(131, 30));
        assert!(activity.is_stopped());
    }
}
