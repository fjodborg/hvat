//! Text measurement utilities.
//!
//! Provides functions for estimating text dimensions based on font metrics.
//! These are approximations used during widget layout.

/// Metrics for a specific font/size combination.
#[derive(Debug, Clone, Copy)]
pub struct TextMetrics {
    /// Font size in pixels
    pub size: f32,
    /// Average character width as a ratio of font size (typically 0.5-0.6 for proportional fonts)
    pub char_width_ratio: f32,
    /// Line height as a ratio of font size (typically 1.2-1.5)
    pub line_height_ratio: f32,
}

impl TextMetrics {
    /// Default metrics for DejaVu Sans Mono (the embedded font).
    /// These ratios are tuned for this specific font.
    pub const MONO: TextMetrics = TextMetrics {
        size: 16.0,
        char_width_ratio: 0.6,      // Monospace: all chars same width
        line_height_ratio: 1.2,
    };

    /// Create metrics for a specific font size.
    pub fn new(size: f32) -> Self {
        Self {
            size,
            char_width_ratio: Self::MONO.char_width_ratio,
            line_height_ratio: Self::MONO.line_height_ratio,
        }
    }

    /// Create metrics with custom ratios.
    pub fn custom(size: f32, char_width_ratio: f32, line_height_ratio: f32) -> Self {
        Self {
            size,
            char_width_ratio,
            line_height_ratio,
        }
    }

    /// Estimate the width of a single line of text.
    pub fn line_width(&self, text: &str) -> f32 {
        // Count visible characters (simple approach)
        let char_count = text.chars().count() as f32;
        char_count * self.size * self.char_width_ratio
    }

    /// Get the line height.
    pub fn line_height(&self) -> f32 {
        self.size * self.line_height_ratio
    }

    /// Estimate dimensions for multi-line text.
    pub fn measure(&self, text: &str) -> (f32, f32) {
        let lines: Vec<&str> = text.lines().collect();

        // Handle empty string: still counts as one line
        let line_count = if lines.is_empty() { 1 } else { lines.len() };

        let width = lines
            .iter()
            .map(|line| self.line_width(line))
            .fold(0.0f32, |max, w| max.max(w));

        let height = line_count as f32 * self.line_height();

        (width, height)
    }

    /// Estimate dimensions for text with a maximum width (with wrapping).
    pub fn measure_wrapped(&self, text: &str, max_width: f32) -> (f32, f32) {
        let lines: Vec<&str> = text.lines().collect();
        let mut total_height: f32 = 0.0;
        let mut actual_width: f32 = 0.0;

        for line in lines {
            let line_w = self.line_width(line);
            if line_w <= max_width {
                actual_width = actual_width.max(line_w);
                total_height += self.line_height();
            } else {
                // Estimate number of wrapped lines
                let num_lines = (line_w / max_width).ceil();
                actual_width = actual_width.max(max_width);
                total_height += num_lines * self.line_height();
            }
        }

        (actual_width, total_height)
    }
}

impl Default for TextMetrics {
    fn default() -> Self {
        Self::MONO
    }
}

/// Convenience function to measure text at a given size.
pub fn measure_text(text: &str, size: f32) -> (f32, f32) {
    TextMetrics::new(size).measure(text)
}

/// Convenience function to get line height at a given size.
pub fn line_height(size: f32) -> f32 {
    TextMetrics::new(size).line_height()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_metrics() {
        let m = TextMetrics::default();
        assert_eq!(m.size, 16.0);
    }

    #[test]
    fn test_line_width() {
        let m = TextMetrics::new(16.0);
        // 5 chars * 16.0 * 0.6 = 48.0
        assert!((m.line_width("hello") - 48.0).abs() < 0.01);
    }

    #[test]
    fn test_line_height() {
        let m = TextMetrics::new(16.0);
        // 16.0 * 1.2 = 19.2
        assert!((m.line_height() - 19.2).abs() < 0.01);
    }

    #[test]
    fn test_multiline_measure() {
        let m = TextMetrics::new(16.0);
        let (w, h) = m.measure("hello\nworld!");
        // Width should be max of lines: "world!" = 6 chars = 57.6
        assert!((w - 57.6).abs() < 0.01);
        // Height should be 2 lines * 19.2 = 38.4
        assert!((h - 38.4).abs() < 0.01);
    }

    #[test]
    fn test_empty_text() {
        let m = TextMetrics::new(16.0);
        let (w, h) = m.measure("");
        assert_eq!(w, 0.0);
        // Empty string has 1 line
        assert!((h - 19.2).abs() < 0.01);
    }
}
