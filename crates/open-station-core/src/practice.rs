use crate::config::PracticeTiming;
use open_station_protocol::types::Mode;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PracticePhase {
    Idle,
    Countdown,
    Autonomous,
    Delay,
    Teleop,
    Done,
}

/// What the practice mode wants the DS to do this tick
#[derive(Debug, Clone)]
pub struct PracticeTick {
    pub phase: PracticePhase,
    pub elapsed: Duration,    // time in current phase
    pub remaining: Duration,  // time left in current phase
    pub should_enable: bool,  // true on transition INTO auto or teleop
    pub should_disable: bool, // true on transition OUT of auto/teleop
    pub mode: Option<Mode>,   // what mode to set (Some only on transitions)
}

pub struct PracticeMode {
    phase: PracticePhase,
    timing: PracticeTiming,
    phase_start: Option<Instant>,
    a_stopped: bool,           // A-Stop active during auto
    prev_phase: PracticePhase, // for detecting transitions
}

impl PracticeMode {
    pub fn new(timing: PracticeTiming) -> Self {
        Self {
            phase: PracticePhase::Idle,
            timing,
            phase_start: None,
            a_stopped: false,
            prev_phase: PracticePhase::Idle,
        }
    }

    pub fn start(&mut self) {
        self.phase = PracticePhase::Countdown;
        self.phase_start = Some(Instant::now());
        self.a_stopped = false;
        self.prev_phase = PracticePhase::Idle;
    }

    pub fn stop(&mut self) {
        self.phase = PracticePhase::Idle;
        self.phase_start = None;
        self.a_stopped = false;
    }

    /// A-Stop: disable during auto, auto-re-enable at teleop start
    pub fn a_stop(&mut self) {
        if self.phase == PracticePhase::Autonomous {
            self.a_stopped = true;
        }
    }

    /// Call every ~20ms. Returns what the DS should do.
    pub fn tick(&mut self) -> PracticeTick {
        let now = Instant::now();
        let elapsed = self
            .phase_start
            .map(|s| now.duration_since(s))
            .unwrap_or_default();

        let phase_duration = self.phase_duration();

        // Check if current phase has expired
        if let Some(dur) = phase_duration {
            if elapsed >= dur {
                self.advance_phase(now);
            }
        }

        let elapsed = self
            .phase_start
            .map(|s| now.duration_since(s))
            .unwrap_or_default();
        let remaining = self
            .phase_duration()
            .map(|d| d.saturating_sub(elapsed))
            .unwrap_or_default();

        let transitioning = self.phase != self.prev_phase;
        let should_enable = transitioning
            && matches!(
                self.phase,
                PracticePhase::Autonomous | PracticePhase::Teleop
            )
            && !self.a_stopped;
        let should_disable = transitioning
            && matches!(
                self.phase,
                PracticePhase::Delay | PracticePhase::Done | PracticePhase::Countdown
            );

        let mode = if transitioning {
            match self.phase {
                PracticePhase::Autonomous => Some(Mode::Autonomous),
                PracticePhase::Teleop => Some(Mode::Teleop),
                _ => None,
            }
        } else {
            None
        };

        // Handle A-Stop: if a_stopped and we just transitioned to teleop, enable
        let should_enable =
            if self.phase == PracticePhase::Teleop && transitioning && self.a_stopped {
                self.a_stopped = false;
                true
            } else {
                should_enable
            };

        // A-Stop should disable during auto
        let should_disable = if self.a_stopped && self.phase == PracticePhase::Autonomous {
            true
        } else {
            should_disable
        };

        self.prev_phase = self.phase;

        PracticeTick {
            phase: self.phase,
            elapsed,
            remaining,
            should_enable,
            should_disable,
            mode,
        }
    }

    pub fn phase(&self) -> PracticePhase {
        self.phase
    }

    pub fn is_running(&self) -> bool {
        self.phase != PracticePhase::Idle && self.phase != PracticePhase::Done
    }

    pub fn set_timing(&mut self, timing: PracticeTiming) {
        self.timing = timing;
    }

    fn phase_duration(&self) -> Option<Duration> {
        match self.phase {
            PracticePhase::Idle => None,
            PracticePhase::Countdown => {
                Some(Duration::from_secs(self.timing.countdown_secs as u64))
            }
            PracticePhase::Autonomous => Some(Duration::from_secs(self.timing.auto_secs as u64)),
            PracticePhase::Delay => Some(Duration::from_secs(self.timing.delay_secs as u64)),
            PracticePhase::Teleop => Some(Duration::from_secs(self.timing.teleop_secs as u64)),
            PracticePhase::Done => None,
        }
    }

    fn advance_phase(&mut self, now: Instant) {
        self.phase = match self.phase {
            PracticePhase::Countdown => PracticePhase::Autonomous,
            PracticePhase::Autonomous => PracticePhase::Delay,
            PracticePhase::Delay => PracticePhase::Teleop,
            PracticePhase::Teleop => PracticePhase::Done,
            other => other, // Idle and Done don't advance
        };
        self.phase_start = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fast_timing() -> PracticeTiming {
        PracticeTiming {
            countdown_secs: 0,
            auto_secs: 0,
            delay_secs: 0,
            teleop_secs: 0,
        }
    }

    #[test]
    fn test_initial_state() {
        let pm = PracticeMode::new(PracticeTiming::default());
        assert_eq!(pm.phase(), PracticePhase::Idle);
        assert!(!pm.is_running());
    }

    #[test]
    fn test_start() {
        let mut pm = PracticeMode::new(PracticeTiming::default());
        pm.start();
        assert_eq!(pm.phase(), PracticePhase::Countdown);
        assert!(pm.is_running());
    }

    #[test]
    fn test_stop_resets_to_idle() {
        let mut pm = PracticeMode::new(PracticeTiming::default());
        pm.start();
        pm.stop();
        assert_eq!(pm.phase(), PracticePhase::Idle);
        assert!(!pm.is_running());
    }

    #[test]
    fn test_phase_transitions_with_zero_timing() {
        // With 0-second timing, phases should advance immediately on tick
        let mut pm = PracticeMode::new(fast_timing());
        pm.start();

        // First tick advances from Countdown
        let tick = pm.tick();
        // Should have advanced past countdown
        // Keep ticking until Done
        let mut phases_seen = vec![tick.phase];
        for _ in 0..10 {
            let tick = pm.tick();
            if !phases_seen.contains(&tick.phase) {
                phases_seen.push(tick.phase);
            }
            if tick.phase == PracticePhase::Done {
                break;
            }
        }
        assert!(phases_seen.contains(&PracticePhase::Done));
        assert!(!pm.is_running());
    }

    #[test]
    fn test_enable_on_auto_transition() {
        let mut pm = PracticeMode::new(fast_timing());
        pm.start();
        // Tick through until we see should_enable with Auto mode
        let mut saw_auto_enable = false;
        for _ in 0..10 {
            let tick = pm.tick();
            if tick.should_enable && tick.mode == Some(Mode::Autonomous) {
                saw_auto_enable = true;
                break;
            }
        }
        assert!(saw_auto_enable);
    }

    #[test]
    fn test_done_is_not_running() {
        let mut pm = PracticeMode::new(fast_timing());
        pm.start();
        for _ in 0..20 {
            pm.tick();
        }
        assert_eq!(pm.phase(), PracticePhase::Done);
        assert!(!pm.is_running());
    }
}
