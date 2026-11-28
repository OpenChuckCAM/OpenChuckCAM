// SPDX-License-Identifier: AGPL-3.0-or-later
//! When the project's own files are saved automatically (PRJ-08; decision
//! PROJUX-B-3, the owner's answer Q3 B with the title-bar switch, relayed).
//!
//! Auto-save is the user's choice, off by default. When it is on, the
//! project is saved as File > Save saves it: at once when it is turned on
//! with changes, then each time the interval has passed since the first
//! change not yet saved. It waits while a box holds unconfirmed text or a
//! dialog is open (decision 0233), and after a recovery until the user has
//! saved once, because only an explicit Save writes a recovered state to
//! the saved files (decision 0243 item 8). A failed save is tried again
//! after the interval, and said once until a save works.
//!
//! The caller supplies monotonic elapsed time and the facts of the frame.
//! This type does no file work; the front end saves and reports back.

use std::time::Duration;

/// The interval's range, in whole minutes, and its default.
pub const MINUTES: (u16, u16, u16) = (1, 5, 30);

/// The facts of one frame that decide whether a save may happen now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Frame {
    /// The user's switch.
    pub on: bool,
    /// The project differs from its saved files.
    pub modified: bool,
    /// The project was recovered from a checkpoint and not saved since.
    pub recovered: bool,
    /// A box holds text that is not confirmed yet.
    pub typing: bool,
    /// A dialog is open in front of the window.
    pub dialog: bool,
}

/// What auto-save does this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The switch is off.
    Off,
    /// Nothing to save, or not yet time.
    Idle,
    /// A save is due but waits for this.
    Waiting(Wait),
    /// Save now.
    Save,
}

/// Why a due save waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    /// A box holds unconfirmed text.
    Typing,
    /// A dialog is open.
    Dialog,
    /// The project was recovered and has not been saved by the user since.
    Recovered,
}

/// The auto-save clock of one open project.
#[derive(Debug, Clone, Default)]
pub struct Schedule {
    /// When the next save is due, from the first change not yet saved.
    due: Option<Duration>,
    /// The switch as it was in the last frame.
    was_on: bool,
    /// The reason the last save failed, until one works.
    failure: Option<String>,
    /// When the last automatic save was made.
    last: Option<Duration>,
}

impl Schedule {
    /// What to do at `now`, with an interval of `minutes` (brought into its
    /// range).
    pub fn step(&mut self, now: Duration, minutes: u16, frame: Frame) -> Step {
        let interval = Duration::from_secs(60 * u64::from(minutes.clamp(MINUTES.0, MINUTES.2)));
        if !frame.on {
            self.was_on = false;
            self.due = None;
            return Step::Off;
        }
        if !self.was_on {
            // Turned on (or opened with it on): a project with changes is
            // saved at once (the owner's point a).
            self.was_on = true;
            self.due = Some(now);
        }
        if !frame.modified {
            // The clock starts again at the next change.
            self.due = None;
            return Step::Idle;
        }
        let due = *self.due.get_or_insert(now + interval);
        if now < due {
            return Step::Idle;
        }
        if frame.recovered {
            Step::Waiting(Wait::Recovered)
        } else if frame.typing {
            Step::Waiting(Wait::Typing)
        } else if frame.dialog {
            Step::Waiting(Wait::Dialog)
        } else {
            Step::Save
        }
    }

    /// The save asked for at `now` worked. Returns true when a failure said
    /// before is now over.
    pub fn saved(&mut self, now: Duration) -> bool {
        self.last = Some(now);
        self.due = None;
        self.failure.take().is_some()
    }

    /// A save the user made (File > Save) also counts: the clock starts
    /// again at the next change, and a failure is over.
    pub fn saved_by_hand(&mut self) -> bool {
        self.due = None;
        self.failure.take().is_some()
    }

    /// The save asked for at `now` failed with `reason`; it is tried again
    /// after `minutes`. Returns true when this failure is new, so it is said
    /// once, not at every try.
    pub fn failed(&mut self, now: Duration, minutes: u16, reason: String) -> bool {
        let interval = Duration::from_secs(60 * u64::from(minutes.clamp(MINUTES.0, MINUTES.2)));
        self.due = Some(now + interval);
        let new = self.failure.is_none();
        self.failure = Some(reason);
        new
    }

    /// Why the last save failed, until one works.
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    /// When the last automatic save was made.
    pub fn last(&self) -> Option<Duration> {
        self.last
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: fn(u64) -> Duration = Duration::from_secs;
    const M: fn(u64) -> Duration = |m| Duration::from_secs(60 * m);

    fn on(modified: bool) -> Frame {
        Frame {
            on: true,
            modified,
            ..Frame::default()
        }
    }

    #[test]
    fn req_prj_08_off_never_saves() {
        let mut s = Schedule::default();
        for t in [0, 60, 600, 6000] {
            let frame = Frame {
                modified: true,
                ..Frame::default()
            };
            assert_eq!(s.step(S(t), 5, frame), Step::Off);
        }
    }

    #[test]
    fn req_prj_08_turning_it_on_with_changes_saves_at_once() {
        let mut s = Schedule::default();
        assert_eq!(s.step(S(10), 5, on(true)), Step::Save);
        assert!(!s.saved(S(10)));
        assert_eq!(s.last(), Some(S(10)));
        // Turned on with nothing to save: nothing happens.
        let mut s = Schedule::default();
        assert_eq!(s.step(S(10), 5, on(false)), Step::Idle);
    }

    #[test]
    fn req_prj_08_it_saves_an_interval_after_the_first_unsaved_change() {
        let mut s = Schedule::default();
        assert_eq!(s.step(S(0), 5, on(false)), Step::Idle);
        // The first change at 1 min: due at 6 min, not before.
        assert_eq!(s.step(M(1), 5, on(true)), Step::Idle);
        assert_eq!(s.step(M(5), 5, on(true)), Step::Idle);
        assert_eq!(s.step(M(6), 5, on(true)), Step::Save);
        s.saved(M(6));
        // Saved: the clock starts again at the next change.
        assert_eq!(s.step(M(7), 5, on(false)), Step::Idle);
        assert_eq!(s.step(M(20), 5, on(true)), Step::Idle);
        assert_eq!(s.step(M(25), 5, on(true)), Step::Save);
    }

    #[test]
    fn req_prj_08_req_ui_07_a_due_save_waits_for_typing_a_dialog_and_a_recovery() {
        let mut s = Schedule::default();
        let mut frame = on(true);
        frame.typing = true;
        assert_eq!(s.step(S(0), 5, frame), Step::Waiting(Wait::Typing));
        frame.typing = false;
        frame.dialog = true;
        assert_eq!(s.step(S(1), 5, frame), Step::Waiting(Wait::Dialog));
        frame.dialog = false;
        frame.recovered = true;
        assert_eq!(s.step(S(2), 5, frame), Step::Waiting(Wait::Recovered));
        // Once nothing holds it, it saves on the next frame.
        frame.recovered = false;
        assert_eq!(s.step(S(3), 5, frame), Step::Save);
    }

    #[test]
    fn req_prj_08_req_nfr_07_a_failure_is_said_once_and_tried_again_after_the_interval() {
        let mut s = Schedule::default();
        assert_eq!(s.step(S(0), 1, on(true)), Step::Save);
        assert!(s.failed(S(0), 1, "read-only".into()));
        assert_eq!(s.failure(), Some("read-only"));
        assert_eq!(s.step(S(30), 1, on(true)), Step::Idle);
        assert_eq!(s.step(S(60), 1, on(true)), Step::Save);
        // The second failure is not new: not said again.
        assert!(!s.failed(S(60), 1, "read-only".into()));
        assert_eq!(s.step(S(120), 1, on(true)), Step::Save);
        // A save that works ends it, and says so once.
        assert!(s.saved(S(120)));
        assert_eq!(s.failure(), None);
        assert!(!s.saved(S(200)));
    }

    #[test]
    fn req_prj_08_a_save_by_hand_restarts_the_clock_and_ends_a_failure() {
        let mut s = Schedule::default();
        assert_eq!(s.step(S(0), 5, on(true)), Step::Save);
        s.failed(S(0), 5, "disk full".into());
        assert!(s.saved_by_hand());
        assert_eq!(s.step(S(10), 5, on(true)), Step::Idle);
        assert_eq!(s.step(S(10) + M(5), 5, on(true)), Step::Save);
    }

    #[test]
    fn req_prj_08_the_interval_is_held_to_one_to_thirty_minutes() {
        let mut s = Schedule::default();
        assert_eq!(s.step(S(0), 5, on(false)), Step::Idle);
        assert_eq!(s.step(S(0), 0, on(true)), Step::Idle);
        assert_eq!(s.step(M(1), 0, on(true)), Step::Save);
        let mut s = Schedule::default();
        assert_eq!(s.step(S(0), 5, on(false)), Step::Idle);
        assert_eq!(s.step(S(0), 99, on(true)), Step::Idle);
        assert_eq!(s.step(M(29), 99, on(true)), Step::Idle);
        assert_eq!(s.step(M(30), 99, on(true)), Step::Save);
        // Turning it off and on again saves at once.
        assert_eq!(s.step(M(31), 5, Frame::default()), Step::Off);
        assert_eq!(s.step(M(32), 5, on(true)), Step::Save);
    }
}
