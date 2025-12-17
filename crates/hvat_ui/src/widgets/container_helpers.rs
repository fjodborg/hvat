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
    // GlobalMousePress is special: it must be sent to ALL children (for blur handling)
    // and should not stop propagation on first handler
    if matches!(event, Event::GlobalMousePress { .. }) {
        let mut result: Option<M> = None;
        for (child, bounds) in children.iter_mut().zip(child_bounds.iter()) {
            let absolute_bounds = translate_bounds(*bounds, container_bounds);
            if let Some(msg) = child.on_event(event, absolute_bounds) {
                // Keep the last message (or first, doesn't matter much for blur events)
                result = Some(msg);
            }
        }
        return result;
    }

    // MouseScroll when any child has an overlay: send to overlay child FIRST so it can close
    // Then let other children handle the scroll normally
    if matches!(event, Event::MouseScroll { .. }) {
        let has_overlay = children.iter().any(|c| c.has_active_overlay());
        if has_overlay {
            // First, dispatch to children with overlays - they get priority
            for (child, bounds) in children.iter_mut().zip(child_bounds.iter()) {
                if child.has_active_overlay() {
                    let absolute_bounds = translate_bounds(*bounds, container_bounds);
                    if let Some(msg) = child.on_event(event, absolute_bounds) {
                        // Overlay handled it (e.g., dropdown closed) - consume event
                        return Some(msg);
                    }
                }
            }
            // Overlay didn't produce a message, fall through to normal handling
        }
    }

    // FocusLost and CursorLeft should also be sent to all children
    if matches!(event, Event::FocusLost | Event::CursorLeft) {
        let mut result: Option<M> = None;
        for (child, bounds) in children.iter_mut().zip(child_bounds.iter()) {
            let absolute_bounds = translate_bounds(*bounds, container_bounds);
            if let Some(msg) = child.on_event(event, absolute_bounds) {
                result = Some(msg);
            }
        }
        return result;
    }

    // TextInput and KeyPress should go to children with active overlays first
    // (e.g., dropdown search needs TextInput, and KeyPress for escape/enter/arrows)
    // If no overlay consumes it, send to all children
    if matches!(event, Event::TextInput { .. } | Event::KeyPress { .. }) {
        // First try children with overlays
        for (child, bounds) in children.iter_mut().zip(child_bounds.iter()) {
            if child.has_active_overlay() {
                let absolute_bounds = translate_bounds(*bounds, container_bounds);
                if let Some(msg) = child.on_event(event, absolute_bounds) {
                    return Some(msg);
                }
            }
        }
        // Then try all children normally
        for (child, bounds) in children.iter_mut().zip(child_bounds.iter()) {
            if child.has_active_overlay() {
                continue; // Already tried
            }
            let absolute_bounds = translate_bounds(*bounds, container_bounds);
            if let Some(msg) = child.on_event(event, absolute_bounds) {
                return Some(msg);
            }
        }
        return None;
    }

    // Check if any child has an active overlay or active drag
    let has_overlay = children.iter().any(|c| c.has_active_overlay());
    let has_drag = children.iter().any(|c| c.has_active_drag());

    // Don't filter by bounds for MouseRelease - children may need to handle
    // release events even if mouse moved outside (e.g., buttons)
    // Also don't filter MouseMove when a child is being dragged
    let should_filter = !matches!(event, Event::MouseRelease { .. })
        && !(matches!(event, Event::MouseMove { .. }) && has_drag);

    if should_filter {
        if let Some(pos) = event.position() {
            if !container_bounds.contains(pos.0, pos.1) {
                // Even if outside bounds, check if any child has an active overlay or drag
                // Overlays (like dropdown popups) and drags need to receive events outside their layout bounds
                if !has_overlay && !has_drag {
                    return None;
                }
            }
        }
    }

    // Phase 1: Dispatch to children with active overlays first
    // This ensures popup clicks are handled before underlying elements
    // For overlays, we check if the event is within the capture bounds (which includes popup area)
    for (child, bounds) in children.iter_mut().zip(child_bounds.iter()) {
        if child.has_active_overlay() {
            let absolute_bounds = translate_bounds(*bounds, container_bounds);

            // Check if event position is within capture bounds (includes overlay area)
            if let Some(pos) = event.position() {
                let capture = child.capture_bounds(absolute_bounds);
                let check_bounds = capture.unwrap_or(absolute_bounds);

                if check_bounds.contains(pos.0, pos.1) {
                    // Event is within overlay's capture area - dispatch and consume
                    if let Some(msg) = child.on_event(event, absolute_bounds) {
                        return Some(msg);
                    }
                    // Even if no message returned, consume the event to prevent passthrough
                    return None;
                }

                // For scroll events outside the overlay, still dispatch to the child
                // so it can close itself (dropdowns should close on scroll outside)
                if matches!(event, Event::MouseScroll { .. }) {
                    if let Some(msg) = child.on_event(event, absolute_bounds) {
                        return Some(msg);
                    }
                }
            } else {
                // Non-positional event (keyboard, etc.) - dispatch normally
                if let Some(msg) = child.on_event(event, absolute_bounds) {
                    return Some(msg);
                }
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
