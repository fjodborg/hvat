/// Defines how a widget's dimension should be sized.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    /// Fill all available space
    Fill,

    /// Shrink to fit content
    Shrink,

    /// Fixed size in pixels
    Units(f32),

    /// Fill a portion of available space (0.0 to 1.0)
    FillPortion(f32),
}

impl Length {
    /// Resolve the length to a concrete size given limits.
    pub fn resolve(&self, available: f32, intrinsic: f32) -> f32 {
        match self {
            Length::Fill => available,
            Length::Shrink => intrinsic,
            Length::Units(px) => *px,
            Length::FillPortion(portion) => available * portion,
        }
    }
}

impl Default for Length {
    fn default() -> Self {
        Length::Shrink
    }
}
