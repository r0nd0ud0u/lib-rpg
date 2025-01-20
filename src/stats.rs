#[derive(Debug, Clone)]
pub struct Stats {
    pub deleteme: String,
}

impl Default for Stats {
    fn default() -> Self {
        Stats {
            deleteme: "".to_owned(),
        }
    }
}
