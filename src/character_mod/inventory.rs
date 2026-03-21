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
    pub fn add_potion(&mut self, name: &str, hp_amount: i64) {
        self.consumables.push(Consumable {
            name: name.to_owned(),
            effects: vec![build_hp_effect(hp_amount, false)],
            consumable_kind: ConsumableKind::Potion,
        });
    }

    pub fn add_small_potion(&mut self) {
        self.add_potion("potion", 20);
    }

    pub fn add_super_potion(&mut self) {
        self.add_potion("super potion", 60);
    }

    pub fn add_hyper_potion(&mut self) {
        self.add_potion("hyper potion", 120);
    }

    pub fn remove_potion(&mut self, name: &str) {
        self.consumables
            .retain(|consumable| consumable.name != name);
    }

    pub fn contains_potion(&self, name: &str) -> bool {
        self.consumables.iter().any(|c| c.name == name)
    }

    pub fn add_equipment(&mut self, equipment: &Equipment) {
        self.equipments
            .entry(equipment.category.to_string())
            .or_default()
            .push(equipment.clone());
    }

    pub fn get_equipped_equipment(&self) -> Vec<Equipment> {
        self.equipments
            .values()
            .flatten()
            .filter(|equipment| equipment.equipped)
            .cloned()
            .collect()
    }
}
