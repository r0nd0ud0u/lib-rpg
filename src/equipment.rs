use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
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
