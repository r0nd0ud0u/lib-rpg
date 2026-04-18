use crate::character_mod::{class::Class, rank::Rank};

#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Loot {
    pub name: String,
    pub kind: LootType,
    pub rank: Rank,
    pub level: i64,
    pub class: Vec<Class>,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LootType {
    #[default]
    Equipment,
    Consumable,
    Material,
    Currency,
}
