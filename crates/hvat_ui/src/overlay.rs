//! Global overlay registry for managing popup-style UI elements
//!
//! This module provides centralized overlay management that handles event routing
//! for dropdowns, tooltips, context menus, and other popup elements.
//!
//! # Overview
//!
//! Overlays register their capture bounds during the draw phase each frame.
//! Before event dispatch, the application checks if the event position is within
//! any registered overlay and sets an `overlay_hint` flag accordingly.
//! Widgets can check this hint to avoid handling events that belong to overlays.

use crate::layout::Bounds;

/// Registry of active overlays, rebuilt each frame during draw.
///
/// The registry is cleared at the start of each render frame, and overlays
/// register themselves during their `draw()` call. This ensures the registry
/// always reflects the current visible state.
#[derive(Default)]
pub struct OverlayRegistry {
    overlays: Vec<OverlayEntry>,
}

/// An entry in the overlay registry
#[derive(Debug, Clone, Copy)]
pub struct OverlayEntry {
    /// Bounds that capture events for this overlay
    pub bounds: Bounds,
    /// Z-order for stacking (higher values are on top)
    pub z_order: u32,
}

impl OverlayRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all registered overlays (called at start of each frame)
    pub fn clear(&mut self) {
        self.overlays.clear();
    }

    /// Register an overlay with default z-order (0)
    pub fn register(&mut self, bounds: Bounds) {
        self.register_with_z_order(bounds, 0);
    }

    /// Register an overlay with explicit z-order
    pub fn register_with_z_order(&mut self, bounds: Bounds, z_order: u32) {
        log::trace!(
            "Overlay registered: bounds=({}, {}, {}, {}), z_order={}",
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            z_order
        );
        self.overlays.push(OverlayEntry { bounds, z_order });
    }

    /// Check if position hits any registered overlay
    pub fn has_overlay_at(&self, x: f32, y: f32) -> bool {
        self.overlays.iter().any(|entry| entry.bounds.contains(x, y))
    }

    /// Get the topmost overlay at a position (highest z-order)
    pub fn get_overlay_at(&self, x: f32, y: f32) -> Option<&OverlayEntry> {
        self.overlays
            .iter()
            .filter(|entry| entry.bounds.contains(x, y))
            .max_by_key(|entry| entry.z_order)
    }

    /// Get the number of registered overlays
    pub fn len(&self) -> usize {
        self.overlays.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.overlays.is_empty()
    }

    /// Iterate over all overlays in z-order (lowest to highest)
    pub fn iter_by_z_order(&self) -> impl Iterator<Item = &OverlayEntry> {
        let mut sorted: Vec<_> = self.overlays.iter().collect();
        sorted.sort_by_key(|e| e.z_order);
        sorted.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_clear() {
        let mut registry = OverlayRegistry::new();
        registry.register(Bounds::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(registry.len(), 1);

        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_has_overlay_at() {
        let mut registry = OverlayRegistry::new();
        registry.register(Bounds::new(10.0, 10.0, 50.0, 50.0));

        assert!(registry.has_overlay_at(30.0, 30.0));
        assert!(!registry.has_overlay_at(0.0, 0.0));
        assert!(!registry.has_overlay_at(100.0, 100.0));
    }

    #[test]
    fn test_z_order() {
        let mut registry = OverlayRegistry::new();
        registry.register_with_z_order(Bounds::new(0.0, 0.0, 100.0, 100.0), 1);
        registry.register_with_z_order(Bounds::new(25.0, 25.0, 50.0, 50.0), 2);

        // Point in both overlays should return higher z-order
        let top = registry.get_overlay_at(50.0, 50.0);
        assert!(top.is_some());
        assert_eq!(top.unwrap().z_order, 2);

        // Point only in first overlay
        let bottom = registry.get_overlay_at(10.0, 10.0);
        assert!(bottom.is_some());
        assert_eq!(bottom.unwrap().z_order, 1);
    }
}
