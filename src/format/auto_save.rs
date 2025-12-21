//! Auto-save manager with debouncing.
//!
//! Provides automatic saving of project data with configurable debounce
//! and minimum interval between saves.

use std::time::Duration;
use web_time::Instant;

/// Manages auto-save timing with debouncing.
///
/// The auto-save system uses two mechanisms to prevent excessive saves:
/// 1. **Debounce delay**: After a change, wait for this duration before saving
///    to batch multiple rapid changes together.
/// 2. **Minimum interval**: Enforce a minimum time between saves even if
///    changes keep occurring.
#[derive(Debug)]
pub struct AutoSaveManager {
    /// Minimum interval between saves.
    save_interval: Duration,

    /// Debounce delay (wait this long after last change before saving).
    debounce_delay: Duration,

    /// Time of last successful save.
    last_save: Option<Instant>,

    /// Time of last change that needs saving.
    last_change: Option<Instant>,

    /// Whether auto-save is enabled.
    enabled: bool,

    /// Whether there are unsaved changes.
    dirty: bool,
}

impl AutoSaveManager {
    /// Default minimum interval between saves (60 seconds).
    pub const DEFAULT_SAVE_INTERVAL: Duration = Duration::from_secs(60);

    /// Default debounce delay (5 seconds).
    pub const DEFAULT_DEBOUNCE_DELAY: Duration = Duration::from_secs(5);

    /// Create a new auto-save manager with default settings.
    pub fn new() -> Self {
        Self {
            save_interval: Self::DEFAULT_SAVE_INTERVAL,
            debounce_delay: Self::DEFAULT_DEBOUNCE_DELAY,
            last_save: None,
            last_change: None,
            enabled: true,
            dirty: false,
        }
    }

    /// Create a disabled auto-save manager.
    pub fn disabled() -> Self {
        let mut manager = Self::new();
        manager.enabled = false;
        manager
    }

    /// Set the minimum interval between saves.
    pub fn with_save_interval(mut self, interval: Duration) -> Self {
        self.save_interval = interval;
        self
    }

    /// Set the debounce delay.
    pub fn with_debounce_delay(mut self, delay: Duration) -> Self {
        self.debounce_delay = delay;
        self
    }

    /// Mark that a change occurred that needs saving.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.last_change = Some(Instant::now());
        log::trace!("Auto-save: marked dirty");
    }

    /// Check if there are unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Check if we should save now.
    ///
    /// Returns true if:
    /// - Auto-save is enabled
    /// - There are unsaved changes
    /// - The debounce delay has passed since the last change
    /// - The minimum save interval has passed since the last save
    pub fn should_save(&self) -> bool {
        if !self.enabled || !self.dirty {
            return false;
        }

        let Some(last_change) = self.last_change else {
            return false;
        };

        // Check debounce delay
        if last_change.elapsed() < self.debounce_delay {
            return false;
        }

        // Check minimum interval (if we've saved before)
        if let Some(last_save) = self.last_save {
            if last_save.elapsed() < self.save_interval {
                return false;
            }
        }

        true
    }

    /// Mark that a save completed successfully.
    pub fn mark_saved(&mut self) {
        self.last_save = Some(Instant::now());
        self.dirty = false;
        self.last_change = None;
        log::trace!("Auto-save: marked saved");
    }

    /// Mark that a save failed.
    ///
    /// This keeps the dirty flag set so we'll try again later.
    pub fn mark_save_failed(&mut self) {
        // Update last_save to prevent immediate retry
        self.last_save = Some(Instant::now());
        // Keep dirty = true so we'll try again
        log::trace!("Auto-save: marked save failed");
    }

    /// Set whether auto-save is enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        log::debug!("Auto-save: enabled = {}", enabled);
    }

    /// Check if auto-save is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get time since last save (if any).
    pub fn time_since_last_save(&self) -> Option<Duration> {
        self.last_save.map(|t| t.elapsed())
    }

    /// Get time since last change (if any).
    pub fn time_since_last_change(&self) -> Option<Duration> {
        self.last_change.map(|t| t.elapsed())
    }

    /// Reset all timing state.
    pub fn reset(&mut self) {
        self.last_save = None;
        self.last_change = None;
        self.dirty = false;
    }
}

impl Default for AutoSaveManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let manager = AutoSaveManager::new();
        assert!(!manager.is_dirty());
        assert!(!manager.should_save());
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_mark_dirty() {
        let mut manager = AutoSaveManager::new();
        manager.mark_dirty();
        assert!(manager.is_dirty());
    }

    #[test]
    fn test_mark_saved() {
        let mut manager = AutoSaveManager::new();
        manager.mark_dirty();
        assert!(manager.is_dirty());

        manager.mark_saved();
        assert!(!manager.is_dirty());
    }

    #[test]
    fn test_disabled() {
        let mut manager = AutoSaveManager::disabled();
        manager.mark_dirty();
        assert!(!manager.should_save());
    }

    #[test]
    fn test_debounce_prevents_immediate_save() {
        let mut manager = AutoSaveManager::new()
            .with_debounce_delay(Duration::from_secs(10))
            .with_save_interval(Duration::ZERO);

        manager.mark_dirty();

        // Should not save immediately due to debounce
        assert!(!manager.should_save());
    }
}
