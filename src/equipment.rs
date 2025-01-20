#[derive(Debug, Clone)]
pub struct Equipment {
    pub deleteme: String,
}

impl Default for Equipment {
    fn default() -> Self {
        Equipment {
            deleteme: "".to_owned(),
        }
    }
}
