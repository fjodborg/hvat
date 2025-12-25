//! Shared text editing utilities for text input widgets
//!
//! This module provides common functionality used by TextInput, NumberInput,
//! and Slider (input field) widgets.

use crate::constants::{CHAR_WIDTH_FACTOR, CURSOR_WIDTH};
use crate::layout::Bounds;
use crate::renderer::{Color, Renderer};

// =============================================================================
// Character Width & Position Calculations
// =============================================================================

/// Calculate approximate character width for a monospace font
#[inline]
pub fn char_width(font_size: f32) -> f32 {
    font_size * CHAR_WIDTH_FACTOR
}

/// Convert an x screen position to a character index in the text
///
/// # Arguments
/// * `x` - The x position in screen coordinates
/// * `content_x` - The x position of the content area start
/// * `font_size` - The font size used for rendering
/// * `text_len` - The length of the text string
///
/// # Returns
/// The character index, clamped to [0, text_len]
pub fn x_to_char_index(x: f32, content_x: f32, font_size: f32, text_len: usize) -> usize {
    let relative_x = x - content_x;
    let cw = char_width(font_size);
    let index = (relative_x / cw).round() as i32;
    index.clamp(0, text_len as i32) as usize
}

/// Calculate the x position of the cursor
///
/// # Arguments
/// * `content_x` - The x position of the content area start
/// * `cursor` - The cursor position (character index)
/// * `font_size` - The font size used for rendering
pub fn cursor_x(content_x: f32, cursor: usize, font_size: f32) -> f32 {
    content_x + cursor as f32 * char_width(font_size)
}

// =============================================================================
// Selection Helpers
// =============================================================================

/// Normalize a selection range so start <= end
#[inline]
pub fn normalize_selection(selection: (usize, usize)) -> (usize, usize) {
    let (start, end) = selection;
    (start.min(end), start.max(end))
}

/// Get the selection anchor for extending selection with shift+arrow keys
/// Returns the existing anchor if selection exists, otherwise the cursor position
fn get_selection_anchor(selection: Option<(usize, usize)>, cursor: usize) -> usize {
    selection.map(|(s, _)| s).unwrap_or(cursor)
}

// =============================================================================
// Text Manipulation
// =============================================================================

/// Convert a character index to a byte index in a UTF-8 string.
/// Returns the byte position of the nth character, or the string length if
/// the index is beyond the string.
#[inline]
fn char_index_to_byte_index(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(text.len())
}

/// Get the number of characters in a string (not bytes).
#[inline]
pub fn char_count(text: &str) -> usize {
    text.chars().count()
}

/// Delete the selected text from a string
///
/// # Arguments
/// * `text` - The text to modify
/// * `selection` - The selection range (start, end) as character indices
///
/// # Returns
/// The cursor position after deletion (the start of the selection) as character index
pub fn delete_selection(text: &mut String, selection: (usize, usize)) -> usize {
    let (start, end) = normalize_selection(selection);
    let start_byte = char_index_to_byte_index(text, start);
    let end_byte = char_index_to_byte_index(text, end);
    text.drain(start_byte..end_byte);
    start
}

/// Insert text at cursor position, optionally deleting selection first
///
/// # Arguments
/// * `text` - The text to modify
/// * `cursor` - Current cursor position (character index)
/// * `selection` - Optional selection range to delete first (character indices)
/// * `insert` - The text to insert
///
/// # Returns
/// The new cursor position (after inserted text) as character index
pub fn insert_text(
    text: &mut String,
    cursor: usize,
    selection: Option<(usize, usize)>,
    insert: &str,
) -> usize {
    let insert_char_pos = if let Some(sel) = selection {
        delete_selection(text, sel)
    } else {
        cursor
    };

    let insert_byte_pos = char_index_to_byte_index(text, insert_char_pos);
    text.insert_str(insert_byte_pos, insert);
    insert_char_pos + insert.chars().count()
}

/// Handle backspace key
///
/// # Arguments
/// * `text` - The text to modify
/// * `cursor` - Current cursor position (character index)
/// * `selection` - Optional selection range (character indices)
///
/// # Returns
/// `Some(new_cursor)` if text was modified, `None` if nothing happened
pub fn handle_backspace(
    text: &mut String,
    cursor: usize,
    selection: Option<(usize, usize)>,
) -> Option<usize> {
    if let Some(sel) = selection {
        Some(delete_selection(text, sel))
    } else if cursor > 0 {
        let prev_char_idx = cursor - 1;
        let byte_idx = char_index_to_byte_index(text, prev_char_idx);
        text.remove(byte_idx);
        Some(prev_char_idx)
    } else {
        None
    }
}

/// Handle delete key
///
/// # Arguments
/// * `text` - The text to modify
/// * `cursor` - Current cursor position (character index)
/// * `selection` - Optional selection range (character indices)
///
/// # Returns
/// `Some(new_cursor)` if text was modified, `None` if nothing happened
pub fn handle_delete(
    text: &mut String,
    cursor: usize,
    selection: Option<(usize, usize)>,
) -> Option<usize> {
    if let Some(sel) = selection {
        Some(delete_selection(text, sel))
    } else if cursor < char_count(text) {
        let byte_idx = char_index_to_byte_index(text, cursor);
        text.remove(byte_idx);
        Some(cursor)
    } else {
        None
    }
}

// =============================================================================
// Cursor Navigation
// =============================================================================

/// Result of a navigation action
pub struct NavResult {
    pub cursor: usize,
    pub selection: Option<(usize, usize)>,
}

/// Handle left arrow key
pub fn handle_left(cursor: usize, selection: Option<(usize, usize)>, shift: bool) -> NavResult {
    if shift {
        // Extend selection
        if cursor > 0 {
            let anchor = get_selection_anchor(selection, cursor);
            NavResult {
                cursor: cursor - 1,
                selection: Some((anchor, cursor - 1)),
            }
        } else {
            NavResult { cursor, selection }
        }
    } else {
        // Normal navigation
        if let Some(sel) = selection {
            let (start, end) = normalize_selection(sel);
            NavResult {
                cursor: start.min(end),
                selection: None,
            }
        } else if cursor > 0 {
            NavResult {
                cursor: cursor - 1,
                selection: None,
            }
        } else {
            NavResult {
                cursor,
                selection: None,
            }
        }
    }
}

/// Handle right arrow key
pub fn handle_right(
    cursor: usize,
    selection: Option<(usize, usize)>,
    text_len: usize,
    shift: bool,
) -> NavResult {
    if shift {
        // Extend selection
        if cursor < text_len {
            let anchor = get_selection_anchor(selection, cursor);
            NavResult {
                cursor: cursor + 1,
                selection: Some((anchor, cursor + 1)),
            }
        } else {
            NavResult { cursor, selection }
        }
    } else {
        // Normal navigation
        if let Some(sel) = selection {
            let (start, end) = normalize_selection(sel);
            NavResult {
                cursor: start.max(end),
                selection: None,
            }
        } else if cursor < text_len {
            NavResult {
                cursor: cursor + 1,
                selection: None,
            }
        } else {
            NavResult {
                cursor,
                selection: None,
            }
        }
    }
}

/// Handle home key
pub fn handle_home(cursor: usize, selection: Option<(usize, usize)>, shift: bool) -> NavResult {
    if shift {
        let anchor = get_selection_anchor(selection, cursor);
        NavResult {
            cursor: 0,
            selection: Some((anchor, 0)),
        }
    } else {
        NavResult {
            cursor: 0,
            selection: None,
        }
    }
}

/// Handle end key
pub fn handle_end(
    cursor: usize,
    selection: Option<(usize, usize)>,
    text_len: usize,
    shift: bool,
) -> NavResult {
    if shift {
        let anchor = get_selection_anchor(selection, cursor);
        NavResult {
            cursor: text_len,
            selection: Some((anchor, text_len)),
        }
    } else {
        NavResult {
            cursor: text_len,
            selection: None,
        }
    }
}

/// Handle select all (Ctrl+A)
pub fn handle_select_all(text_len: usize) -> NavResult {
    NavResult {
        cursor: text_len,
        selection: Some((0, text_len)),
    }
}

// =============================================================================
// Rendering Helpers
// =============================================================================

/// Draw a text selection highlight
///
/// # Arguments
/// * `renderer` - The renderer to draw with
/// * `content` - The content bounds
/// * `selection` - The selection range (start, end)
/// * `font_size` - The font size
/// * `color` - The selection highlight color
pub fn draw_selection(
    renderer: &mut Renderer,
    content: Bounds,
    selection: (usize, usize),
    font_size: f32,
    color: Color,
) {
    let (start, end) = normalize_selection(selection);
    let cw = char_width(font_size);
    let sel_x = content.x + start as f32 * cw;
    let sel_width = (end - start) as f32 * cw;
    let sel_bounds = Bounds::new(sel_x, content.y, sel_width, content.height);
    renderer.fill_rect(sel_bounds, color);
}

/// Draw a text cursor
///
/// # Arguments
/// * `renderer` - The renderer to draw with
/// * `content` - The content bounds
/// * `cursor` - The cursor position (character index)
/// * `font_size` - The font size
/// * `color` - The cursor color
pub fn draw_cursor(
    renderer: &mut Renderer,
    content: Bounds,
    cursor: usize,
    font_size: f32,
    color: Color,
) {
    let x = cursor_x(content.x, cursor, font_size);
    let cursor_bounds = Bounds::new(x, content.y + 2.0, CURSOR_WIDTH, content.height - 4.0);
    renderer.fill_rect(cursor_bounds, color);
}

// =============================================================================
// Input Field Drawing Helpers
// =============================================================================

use crate::layout::Padding;
use crate::widgets::config::BaseInputConfig;

/// Calculate content bounds from outer bounds and padding
///
/// This is the standard formula used by text input widgets to get the
/// drawable content area inside padding.
pub fn content_bounds(bounds: Bounds, padding: &Padding) -> Bounds {
    Bounds::new(
        bounds.x + padding.left,
        bounds.y + padding.top,
        bounds.width - padding.horizontal(),
        bounds.height - padding.vertical(),
    )
}

/// Corner radius for input fields (modern SaaS style)
const INPUT_CORNER_RADIUS: f32 = 4.0;

/// Draw standard input field background and border
///
/// Draws the background fill and border stroke based on focus state,
/// using colors from the BaseInputConfig. Uses rounded corners for modern look.
pub fn draw_input_background(
    renderer: &mut Renderer,
    bounds: Bounds,
    is_focused: bool,
    config: &BaseInputConfig,
) {
    renderer.fill_rounded_rect(bounds, config.background(is_focused), INPUT_CORNER_RADIUS);
    renderer.stroke_rounded_rect(bounds, config.border(is_focused), INPUT_CORNER_RADIUS, 1.0);
}

// =============================================================================
// Focus/Blur Handling Helpers
// =============================================================================

/// Handle blur when clicking outside the input bounds
///
/// Returns true if focus state changed (was focused and clicked outside).
pub fn handle_blur_on_outside_click(
    is_focused: &mut bool,
    selection: &mut Option<(usize, usize)>,
    position: (f32, f32),
    bounds: Bounds,
) -> bool {
    if *is_focused {
        let (x, y) = position;
        if !bounds.contains(x, y) {
            *is_focused = false;
            *selection = None;
            return true;
        }
    }
    false
}

/// Handle blur when window loses focus
///
/// Returns true if focus state changed (was focused).
pub fn handle_focus_lost(is_focused: &mut bool, selection: &mut Option<(usize, usize)>) -> bool {
    if *is_focused {
        *is_focused = false;
        *selection = None;
        return true;
    }
    false
}

// =============================================================================
// Number Input Validation
// =============================================================================

/// Check if a character is valid for number input
///
/// # Arguments
/// * `c` - The character to check
/// * `cursor` - Current cursor position
/// * `text` - Current text content
///
/// # Returns
/// `true` if the character is valid at this position
pub fn is_valid_number_char(c: char, cursor: usize, text: &str) -> bool {
    // Digits are always valid
    if c.is_ascii_digit() {
        return true;
    }

    // Minus only at start
    if c == '-' && cursor == 0 {
        return true;
    }

    // Only one decimal point
    if c == '.' && !text.contains('.') {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_width() {
        assert!((char_width(14.0) - 8.4).abs() < 0.01);
    }

    #[test]
    fn test_x_to_char_index() {
        // At content start
        assert_eq!(x_to_char_index(100.0, 100.0, 14.0, 10), 0);
        // One character in
        assert_eq!(x_to_char_index(108.4, 100.0, 14.0, 10), 1);
        // Beyond text end
        assert_eq!(x_to_char_index(200.0, 100.0, 14.0, 5), 5);
        // Before content start
        assert_eq!(x_to_char_index(50.0, 100.0, 14.0, 10), 0);
    }

    #[test]
    fn test_normalize_selection() {
        assert_eq!(normalize_selection((5, 10)), (5, 10));
        assert_eq!(normalize_selection((10, 5)), (5, 10));
        assert_eq!(normalize_selection((5, 5)), (5, 5));
    }

    #[test]
    fn test_delete_selection() {
        let mut text = String::from("Hello World");
        let cursor = delete_selection(&mut text, (6, 11));
        assert_eq!(text, "Hello ");
        assert_eq!(cursor, 6);

        let mut text = String::from("Hello World");
        let cursor = delete_selection(&mut text, (0, 6));
        assert_eq!(text, "World");
        assert_eq!(cursor, 0);
    }

    #[test]
    fn test_insert_text() {
        let mut text = String::from("Hello World");
        let cursor = insert_text(&mut text, 5, None, " Beautiful");
        assert_eq!(text, "Hello Beautiful World");
        assert_eq!(cursor, 15);

        // With selection replacement
        let mut text = String::from("Hello World");
        let cursor = insert_text(&mut text, 6, Some((6, 11)), "Universe");
        assert_eq!(text, "Hello Universe");
        assert_eq!(cursor, 14);
    }

    #[test]
    fn test_handle_backspace() {
        let mut text = String::from("Hello");
        assert_eq!(handle_backspace(&mut text, 5, None), Some(4));
        assert_eq!(text, "Hell");

        let mut text = String::from("Hello");
        assert_eq!(handle_backspace(&mut text, 0, None), None);
        assert_eq!(text, "Hello");

        let mut text = String::from("Hello World");
        assert_eq!(handle_backspace(&mut text, 6, Some((0, 6))), Some(0));
        assert_eq!(text, "World");
    }

    #[test]
    fn test_handle_delete() {
        let mut text = String::from("Hello");
        assert_eq!(handle_delete(&mut text, 0, None), Some(0));
        assert_eq!(text, "ello");

        let mut text = String::from("Hello");
        assert_eq!(handle_delete(&mut text, 5, None), None);
        assert_eq!(text, "Hello");
    }

    #[test]
    fn test_handle_left() {
        // Normal left
        let result = handle_left(5, None, false);
        assert_eq!(result.cursor, 4);
        assert!(result.selection.is_none());

        // Left at start
        let result = handle_left(0, None, false);
        assert_eq!(result.cursor, 0);

        // Left with selection - jumps to start
        let result = handle_left(8, Some((3, 8)), false);
        assert_eq!(result.cursor, 3);
        assert!(result.selection.is_none());

        // Shift+Left extends selection
        let result = handle_left(5, None, true);
        assert_eq!(result.cursor, 4);
        assert_eq!(result.selection, Some((5, 4)));
    }

    #[test]
    fn test_handle_right() {
        let result = handle_right(5, None, 10, false);
        assert_eq!(result.cursor, 6);
        assert!(result.selection.is_none());

        // Right at end
        let result = handle_right(10, None, 10, false);
        assert_eq!(result.cursor, 10);

        // Shift+Right extends selection
        let result = handle_right(5, None, 10, true);
        assert_eq!(result.cursor, 6);
        assert_eq!(result.selection, Some((5, 6)));
    }

    #[test]
    fn test_handle_select_all() {
        let result = handle_select_all(10);
        assert_eq!(result.cursor, 10);
        assert_eq!(result.selection, Some((0, 10)));
    }

    #[test]
    fn test_is_valid_number_char() {
        // Digits always valid
        assert!(is_valid_number_char('5', 0, ""));
        assert!(is_valid_number_char('5', 3, "123"));

        // Minus only at start
        assert!(is_valid_number_char('-', 0, ""));
        assert!(!is_valid_number_char('-', 1, "1"));

        // Only one decimal point
        assert!(is_valid_number_char('.', 1, "1"));
        assert!(!is_valid_number_char('.', 2, "1."));

        // Other chars invalid
        assert!(!is_valid_number_char('a', 0, ""));
        assert!(!is_valid_number_char('+', 0, ""));
    }
}
