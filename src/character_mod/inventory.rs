use std::collections::HashMap;

use crate::character_mod::{
    effect::{EffectParam, build_hp_effect},
    equipment::Equipment,
};

#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Inventory {
    pub equipments: HashMap<String, Vec<Equipment>>,
    pub consumables: Vec<Consumable>,
    pub money: u64,
}

#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Consumable {
    pub name: String,
    pub effects: Vec<EffectParam>,
    pub consumable_kind: ConsumableKind,
}

#[repr(usize)]
#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub enum ConsumableKind {
    #[default]
    Potion,
}

impl Inventory {
    pub fn add_potion(&mut self) {
        self.consumables.push(Consumable {
            name: "potion".to_owned(),
            effects: vec![build_hp_effect(20, false)],
            consumable_kind: ConsumableKind::Potion,
        });
    }

    pub fn add_super_potion(&mut self) {
        self.consumables.push(Consumable {
            name: "potion".to_owned(),
            effects: vec![build_hp_effect(60, false)],
            consumable_kind: ConsumableKind::Potion,
        });
    }

    pub fn add_hyper_potion(&mut self) {
        self.consumables.push(Consumable {
            name: "potion".to_owned(),
            effects: vec![build_hp_effect(120, false)],
            consumable_kind: ConsumableKind::Potion,
        });
    }
}
