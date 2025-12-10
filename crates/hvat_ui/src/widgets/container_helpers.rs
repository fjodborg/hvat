//! Helper functions for container layout widgets (Row, Column)

use crate::element::Element;
use crate::event::Event;
use crate::layout::Bounds;

/// Dispatch an event to child elements with overlay-aware priority.
///
/// This function handles the common event dispatch pattern used by container widgets:
/// 1. Check if event should be filtered based on bounds
/// 2. First dispatch to children with active overlays (e.g., open dropdowns)
/// 3. Then dispatch to remaining children normally
///
/// # Arguments
/// * `children` - Mutable slice of child elements
/// * `child_bounds` - Slice of cached bounds for each child (relative to container)
/// * `event` - The event to dispatch
/// * `container_bounds` - Absolute bounds of the container
///
/// # Returns
/// * `Some(M)` if a child consumed the event and returned a message
/// * `None` if no child consumed the event
pub fn dispatch_event_to_children<M: 'static>(
    children: &mut [Element<M>],
    child_bounds: &[Bounds],
    event: &Event,
    container_bounds: Bounds,
) -> Option<M> {
    // Check if any child has an active overlay
    let has_overlay = children.iter().any(|c| c.has_active_overlay());

    // Don't filter by bounds for MouseRelease - children may need to handle
    // release events even if mouse moved outside (e.g., buttons)
    let should_filter = !matches!(event, Event::MouseRelease { .. });

    if should_filter {
        if let Some(pos) = event.position() {
            if !container_bounds.contains(pos.0, pos.1) {
                // Even if outside bounds, check if any child has an active overlay
                // Overlays (like dropdown popups) need to receive events outside their layout bounds
                if !has_overlay {
                    return None;
                }
            }
        }
    }

    // Phase 1: Dispatch to children with active overlays first
    // This ensures popup clicks are handled before underlying elements
    for (child, bounds) in children.iter_mut().zip(child_bounds.iter()) {
        if child.has_active_overlay() {
            let absolute_bounds = translate_bounds(*bounds, container_bounds);
            if let Some(msg) = child.on_event(event, absolute_bounds) {
                return Some(msg);
            }
        }
    }

    // Phase 2: Dispatch to remaining children normally
    for (child, bounds) in children.iter_mut().zip(child_bounds.iter()) {
        if child.has_active_overlay() {
            continue; // Already handled above
        }

        let absolute_bounds = translate_bounds(*bounds, container_bounds);
        if let Some(msg) = child.on_event(event, absolute_bounds) {
            return Some(msg);
        }
    }

    None
}

/// Translate relative child bounds to absolute bounds within the container.
#[inline]
fn translate_bounds(child_bounds: Bounds, container_bounds: Bounds) -> Bounds {
    Bounds::new(
        container_bounds.x + child_bounds.x,
        container_bounds.y + child_bounds.y,
        child_bounds.width,
        child_bounds.height,
    )
}

/// Draw children at their cached bounds positions.
///
/// # Arguments
/// * `children` - Slice of child elements to draw
/// * `child_bounds` - Slice of cached bounds for each child (relative to container)
/// * `renderer` - The renderer to draw with
/// * `container_bounds` - Absolute bounds of the container
pub fn draw_children<M: 'static>(
    children: &[Element<M>],
    child_bounds: &[Bounds],
    renderer: &mut crate::renderer::Renderer,
    container_bounds: Bounds,
) {
    for (child, bounds) in children.iter().zip(child_bounds.iter()) {
        let absolute_bounds = translate_bounds(*bounds, container_bounds);
        child.draw(renderer, absolute_bounds);
    }
}
