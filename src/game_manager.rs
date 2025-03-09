#[derive(Debug, Clone)]
pub struct GameManager {
    pub deleteme: String,
}

impl Default for GameManager {
    fn default() -> Self {
        GameManager {
            deleteme: "".to_owned(),
        }
    }
}
