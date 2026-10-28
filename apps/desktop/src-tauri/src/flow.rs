//! Back-pressure between a shell and the terminal that draws it.
//!
//! The reader thread counts the bytes it sends and the webview acknowledges what xterm has finished
//! parsing. Past a high-water mark the reader waits; it resumes below a low-water mark. A waiting
//! reader stops draining the bounded PTY channel, so the child blocks on its own `write` instead of
//! the webview queueing an unbounded amount of output.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

/// Unacknowledged bytes at which the reader pauses.
pub const HIGH_WATER: usize = 1024 * 1024;
/// Unacknowledged bytes at which a paused reader resumes.
pub const LOW_WATER: usize = 256 * 1024;
/// How long a paused reader tolerates silence from the webview before it gives up on flow control,
/// so a stuck or reloaded page can never freeze a shell for good.
const STALL_LIMIT: Duration = Duration::from_secs(10);
/// A paused reader wakes this often to notice a destroyed session.
const POLL: Duration = Duration::from_millis(100);

pub struct FlowControl {
    pending: Mutex<usize>,
    relieved: Condvar,
    high: usize,
    low: usize,
    stall: Duration,
}

impl Default for FlowControl {
    fn default() -> Self {
        Self::new(HIGH_WATER, LOW_WATER, STALL_LIMIT)
    }
}

impl FlowControl {
    pub fn new(high: usize, low: usize, stall: Duration) -> Self {
        Self {
            pending: Mutex::new(0),
            relieved: Condvar::new(),
            high,
            low,
            stall,
        }
    }

    /// Counts bytes that were sent to the webview.
    pub fn sent(&self, bytes: usize) {
        if let Ok(mut pending) = self.pending.lock() {
            *pending = pending.saturating_add(bytes);
        }
    }

    /// Counts bytes the webview has finished with, and wakes a paused reader.
    pub fn acknowledge(&self, bytes: usize) {
        if let Ok(mut pending) = self.pending.lock() {
            *pending = pending.saturating_sub(bytes);
        }
        self.relieved.notify_all();
    }

    /// Wakes a paused reader so it can notice that the session was destroyed.
    pub fn wake(&self) {
        self.relieved.notify_all();
    }

    #[cfg(test)]
    pub fn pending(&self) -> usize {
        self.pending.lock().map(|pending| *pending).unwrap_or(0)
    }

    /// Returns at once while the webview keeps up. Past the high-water mark it waits for
    /// acknowledgements to bring the backlog under the low-water mark, for `cancelled` to be set, or
    /// for the webview to stay silent for the stall limit (then it forgets the backlog and carries on).
    pub fn wait_if_saturated(&self, cancelled: &AtomicBool) {
        let Ok(mut pending) = self.pending.lock() else {
            return;
        };
        if *pending < self.high {
            return;
        }
        let mut last_seen = *pending;
        let mut last_progress = Instant::now();
        while *pending > self.low && !cancelled.load(Ordering::Relaxed) {
            let Ok((guard, _)) = self.relieved.wait_timeout(pending, POLL) else {
                return;
            };
            pending = guard;
            if *pending != last_seen {
                last_seen = *pending;
                last_progress = Instant::now();
            } else if last_progress.elapsed() >= self.stall {
                *pending = 0;
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;

    use super::*;

    fn flow() -> Arc<FlowControl> {
        Arc::new(FlowControl::new(100, 40, Duration::from_secs(5)))
    }

    #[test]
    fn it_does_not_wait_while_the_webview_keeps_up() {
        let flow = flow();
        flow.sent(99);
        let started = Instant::now();
        flow.wait_if_saturated(&AtomicBool::new(false));
        assert!(started.elapsed() < Duration::from_millis(50));
    }

    #[test]
    fn it_waits_at_the_high_water_mark_and_resumes_below_the_low_one() {
        let flow = flow();
        flow.sent(100);

        let acker = {
            let flow = flow.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(150));
                flow.acknowledge(30); // 70 pending: still above the low mark
                thread::sleep(Duration::from_millis(150));
                flow.acknowledge(40); // 30 pending: below it
            })
        };

        let started = Instant::now();
        flow.wait_if_saturated(&AtomicBool::new(false));
        let waited = started.elapsed();
        acker.join().unwrap();

        assert!(
            waited >= Duration::from_millis(280),
            "resumed too early: {waited:?}"
        );
        assert_eq!(flow.pending(), 30);
    }

    #[test]
    fn a_destroyed_session_releases_a_waiting_reader() {
        let flow = flow();
        flow.sent(500);
        let cancelled = Arc::new(AtomicBool::new(false));

        let waiter = {
            let (flow, cancelled) = (flow.clone(), cancelled.clone());
            thread::spawn(move || flow.wait_if_saturated(&cancelled))
        };
        thread::sleep(Duration::from_millis(100));
        cancelled.store(true, Ordering::Relaxed);
        flow.wake();

        let started = Instant::now();
        waiter.join().unwrap();
        assert!(started.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn a_silent_webview_cannot_freeze_the_shell_for_good() {
        let flow = FlowControl::new(100, 40, Duration::from_millis(300));
        flow.sent(1_000);

        let started = Instant::now();
        flow.wait_if_saturated(&AtomicBool::new(false));

        assert!(started.elapsed() >= Duration::from_millis(300));
        assert_eq!(
            flow.pending(),
            0,
            "the backlog is forgotten once the webview stalls"
        );
    }

    #[test]
    fn acknowledging_more_than_was_sent_never_underflows() {
        let flow = flow();
        flow.sent(10);
        flow.acknowledge(1_000);
        assert_eq!(flow.pending(), 0);
    }
}
