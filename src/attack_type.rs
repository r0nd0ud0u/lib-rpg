use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttackType {
    pub name: String,
    /* pub level: u8,
    pub mana_cost: u32,
    pub vigor_cost: u32,
    pub berseck_cost: u32,
    pub target: String,
    pub reach: String,
    pub name_photo: String,
    pub all_effects: Vec<EffectParam>,
    pub form: String, */
}

impl Default for AttackType {
    fn default() -> Self {
        AttackType {
            name: "".to_owned(),
        }
    }
}
