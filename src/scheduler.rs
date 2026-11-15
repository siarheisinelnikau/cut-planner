use std::time::{Duration, Instant};

/// Reactive scheduling: dirty state survives minimization, cosmetic deadlines do not.
#[derive(Debug)]
pub struct Scheduler {
    pub visible: bool,
    pub dirty: bool,
    pub deadline: Option<Instant>,
}
impl Default for Scheduler {
    fn default() -> Self {
        Self {
            visible: true,
            dirty: true,
            deadline: None,
        }
    }
}
impl Scheduler {
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }
    pub fn visibility(&mut self, visible: bool) {
        self.visible = visible;
        self.deadline = None;
        if visible {
            self.dirty = true;
        }
    }
    pub fn schedule(&mut self, now: Instant, delay: Duration) {
        self.deadline = if self.visible {
            now.checked_add(delay)
        } else {
            None
        };
    }
    pub fn wake_deadline(&mut self, now: Instant) -> bool {
        if self.visible && self.deadline.is_some_and(|d| d <= now) {
            self.deadline = None;
            self.dirty = true;
            true
        } else {
            false
        }
    }
    pub fn take_scene(&mut self) -> bool {
        if self.visible && self.dirty {
            self.dirty = false;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idle_stops_and_hidden_data_waits_for_restore() {
        let mut s = Scheduler::default();
        let now = Instant::now();
        assert!(s.take_scene());
        assert!(!s.take_scene());
        s.schedule(now, Duration::from_millis(120));
        assert!(!s.wake_deadline(now));
        assert!(s.wake_deadline(now + Duration::from_millis(121)));
        assert!(s.take_scene());
        assert!(!s.wake_deadline(now + Duration::from_secs(1)));
        s.visibility(false);
        s.invalidate();
        s.schedule(now, Duration::ZERO);
        assert!(s.deadline.is_none());
        assert!(!s.take_scene());
        s.visibility(true);
        assert!(s.take_scene());
        assert!(!s.take_scene());
    }
}
