use std::collections::HashSet;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    EStop,
    Disable,
    Enable,
    AStop,
    RescanJoysticks,
}

pub struct HotkeyManager {
    tx: mpsc::UnboundedSender<HotkeyAction>,
    rx: mpsc::UnboundedReceiver<HotkeyAction>,
    running: Arc<AtomicBool>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            rx,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start listening for global hotkeys on a background thread
    pub fn start(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            return;
        }
        self.running.store(true, Ordering::SeqCst);

        let tx = self.tx.clone();
        let running = self.running.clone();
        let pressed_keys: Arc<Mutex<HashSet<rdev::Key>>> = Arc::new(Mutex::new(HashSet::new()));
        let keys = pressed_keys.clone();

        thread::spawn(move || {
            let callback = move |event: rdev::Event| {
                match event.event_type {
                    rdev::EventType::KeyPress(key) => {
                        let mut pressed = keys.lock().unwrap();
                        pressed.insert(key);

                        match key {
                            rdev::Key::Space => {
                                let _ = tx.send(HotkeyAction::EStop);
                            }
                            rdev::Key::LeftBracket
                            | rdev::Key::RightBracket
                            | rdev::Key::BackSlash => {
                                // Check if all three enable keys are pressed
                                if pressed.contains(&rdev::Key::LeftBracket)
                                    && pressed.contains(&rdev::Key::RightBracket)
                                    && pressed.contains(&rdev::Key::BackSlash)
                                {
                                    let _ = tx.send(HotkeyAction::Enable);
                                } else {
                                    let _ = tx.send(HotkeyAction::Disable);
                                }
                            }
                            _ => {
                                let _ = tx.send(HotkeyAction::Disable);
                            }
                        }
                    }
                    rdev::EventType::KeyRelease(key) => {
                        let mut pressed = keys.lock().unwrap();
                        pressed.remove(&key);
                    }
                    _ => {}
                }
            };

            // rdev::listen blocks the thread
            if let Err(e) = rdev::listen(callback) {
                log::error!("Global hotkey listener error: {:?}", e);
            }
            running.store(false, Ordering::SeqCst);
        });
    }

    /// Get the next hotkey action (non-blocking)
    pub async fn next_action(&mut self) -> Option<HotkeyAction> {
        self.rx.recv().await
    }

    /// Try to get a hotkey action without waiting
    pub fn try_next_action(&mut self) -> Option<HotkeyAction> {
        self.rx.try_recv().ok()
    }

    /// Stop the hotkey listener
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Default for HotkeyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_action_eq() {
        assert_eq!(HotkeyAction::EStop, HotkeyAction::EStop);
        assert_ne!(HotkeyAction::EStop, HotkeyAction::Disable);
    }

    #[test]
    fn test_manager_creation() {
        let manager = HotkeyManager::new();
        assert!(!manager.is_running());
    }

    #[test]
    fn test_try_next_action_empty() {
        let mut manager = HotkeyManager::new();
        assert!(manager.try_next_action().is_none());
    }
}
