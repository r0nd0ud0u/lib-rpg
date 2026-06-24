#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LogData {
    pub message: String,
    pub color: String,
}

pub mod const_colors {
    pub const DARK_RED: &str = "#9b1c1c";
    pub const LIGHT_GREEN: &str = "#10b981";
    pub const LIGHT_BLUE: &str = "#1a73e8";
    /// Neutral grey for informational messages (effect expiry, etc.)
    pub const MUTED_GREY: &str = "#9ca3af";
}
