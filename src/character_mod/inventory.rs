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

    pub fn remove_equipment(&mut self, equipment_name: &str) {
        for equipments in self.equipments.values_mut() {
            equipments.retain(|equipment| equipment.unique_name != equipment_name);
        }
    }

    pub fn sum_all_equipped_equipment_stat(&self, stat_name: &str) -> (i64, i64) {
        self.get_equipped_equipment()
            .iter()
            .map(|equipment| {
                equipment
                    .stats
                    .all_stats
                    .get(stat_name)
                    .map(|attr| (attr.buf_equip_value, attr.buf_equip_percent))
                    .unwrap_or((0, 0))
            })
            .fold((0, 0), |acc, x| (acc.0 + x.0, acc.1 + x.1))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        character_mod::{
            equipment::{Equipment, EquipmentJsonKey},
            inventory::Inventory,
        },
        common::constants::stats_const::{HP, PHYSICAL_POWER},
        testing::testing_all_characters::testing_test_ally1_vs_test_boss1,
    };

    #[test]
    fn unit_add_and_remove_potion() {
        let mut inventory = Inventory::default();
        inventory.add_small_potion();
        inventory.add_super_potion();
        inventory.add_hyper_potion();

        assert!(inventory.contains_potion("potion"));
        assert!(inventory.contains_potion("super potion"));
        assert!(inventory.contains_potion("hyper potion"));

        inventory.remove_potion("super potion");
        assert!(!inventory.contains_potion("super potion"));
    }

    #[test]
    fn unit_add_and_remove_equipment() {
        let mut inventory = Inventory::default();
        let equipment1 = Equipment {
            name: "Sword of Testing".to_owned(),
            unique_name: "sword_of_testing".to_owned(),
            category: EquipmentJsonKey::LeftWeapon,
            stats: crate::character_mod::stats::Stats::default(),
            equipped: true,
        };
        let equipment2 = Equipment {
            name: "Shield of Testing".to_owned(),
            unique_name: "shield_of_testing".to_owned(),
            category: EquipmentJsonKey::Chest,
            stats: crate::character_mod::stats::Stats::default(),
            equipped: false,
        };
        inventory.add_equipment(&equipment1);
        inventory.add_equipment(&equipment2);
        assert_eq!(inventory.get_equipped_equipment().len(), 1);
        assert_eq!(
            inventory.get_equipped_equipment()[0].name,
            "Sword of Testing"
        );

        inventory.remove_equipment("sword_of_testing");
        assert!(inventory.get_equipped_equipment().is_empty());
    }

    #[test]
    fn unit_sum_all_equipped_equipment_stat() {
        let mut inventory = Inventory::default();
        let mut equipment1 = Equipment {
            name: "Helmet of Testing".to_owned(),
            unique_name: "helmet_of_testing".to_owned(),
            category: EquipmentJsonKey::Head,
            stats: crate::character_mod::stats::Stats::default(),
            equipped: true,
        };
        equipment1.stats.all_stats.insert(
            HP.to_owned(),
            crate::character_mod::stats::Attribute {
                buf_equip_value: 10,
                buf_equip_percent: 10,
                ..Default::default()
            },
        );
        let mut equipment2 = Equipment {
            name: "Armor of Testing".to_owned(),
            unique_name: "armor_of_testing".to_owned(),
            category: EquipmentJsonKey::Chest,
            stats: crate::character_mod::stats::Stats::default(),
            equipped: true,
        };
        equipment2.stats.all_stats.insert(
            HP.to_owned(),
            crate::character_mod::stats::Attribute {
                buf_equip_value: 20,
                buf_equip_percent: 20,
                ..Default::default()
            },
        );
        inventory.add_equipment(&equipment1);
        inventory.add_equipment(&equipment2);
        assert_eq!(inventory.sum_all_equipped_equipment_stat(HP), (30, 30));

        // test with character
        let (gm, _hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();
        gm.pm
            .current_player
            .inventory
            .sum_all_equipped_equipment_stat(PHYSICAL_POWER);
        assert_eq!(
            gm.pm
                .current_player
                .inventory
                .sum_all_equipped_equipment_stat(PHYSICAL_POWER),
            (30, 0)
        );
    }
}
