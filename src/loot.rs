/// Defines the parameters for a loot
#[derive(Debug, Clone)]
pub struct Loot {
    pub deleteme: String,
}

#[cfg(not(test))]
impl Default for Loot {
    fn default() -> Self {
        Loot {
            deleteme: "".to_owned(),
        }
    }
}
