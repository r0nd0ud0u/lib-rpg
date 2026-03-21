use std::collections::HashMap;

use strum::IntoEnumIterator;

use crate::character_mod::{
    effect::{EffectParam, build_hp_effect},
    equipment::{Equipment, EquipmentJsonKey},
};

#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Inventory {
    pub equipments: HashMap<String, Vec<EquipmentInventory>>, // key: equipment category, value: list of equipment unique name
    pub consumables: Vec<Consumable>,
    pub money: u64,
}

#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct EquipmentInventory {
    pub unique_name: String,
    pub is_equipped: bool,
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

    pub fn add_equipment(&mut self, equipment: &Equipment, is_equipped: bool) {
        self.equipments
            .entry(equipment.category.to_string())
            .or_default()
            .push(EquipmentInventory {
                unique_name: equipment.unique_name.clone(),
                is_equipped,
            });
    }

    pub fn get_all_equipments(
        &self,
        all_equipments: &[Equipment],
        is_equipped_filter: bool,
    ) -> HashMap<String, Vec<Equipment>> {
        let mut equipped_map: HashMap<String, Vec<Equipment>> = HashMap::new();
        for e in EquipmentJsonKey::iter() {
            let equipped_equipments = self
                .equipments
                .get(&e.to_string())
                .map(|unique_names| {
                    unique_names
                        .iter()
                        .filter_map(|equipment_inventory| {
                            all_equipments.iter().find(|equipment| {
                                (!is_equipped_filter || equipment_inventory.is_equipped)
                                    && equipment.unique_name == equipment_inventory.unique_name
                            })
                        })
                        .cloned()
                        .collect::<Vec<Equipment>>()
                })
                .unwrap_or_default();
            equipped_map.insert(e.to_string(), equipped_equipments);
        }
        equipped_map
    }

    pub fn get_equipped_equipments(
        &self,
        all_equipments: &[Equipment],
    ) -> HashMap<String, Vec<Equipment>> {
        self.get_all_equipments(all_equipments, true)
    }

    pub fn get_equipment_by_name(
        &self,
        unique_name: &str,
        all_equipments: &[Equipment],
    ) -> Option<Equipment> {
        all_equipments
            .iter()
            .find(|equipment| equipment.unique_name == unique_name)
            .cloned()
    }

    pub fn remove_equipment(&mut self, equipment_unique_name: &str) {
        for equipments in self.equipments.values_mut() {
            equipments.retain(|equipment| equipment.unique_name != equipment_unique_name);
        }
    }

    pub fn sum_all_equipped_equipment_stat(
        &self,
        stat_name: &str,
        list_equipments: &[Equipment],
    ) -> (i64, i64) {
        self.get_equipped_equipments(list_equipments)
            .values()
            .flatten()
            .cloned()
            .collect::<Vec<Equipment>>()
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
        };
        let equipment2 = Equipment {
            name: "Shield of Testing".to_owned(),
            unique_name: "shield_of_testing".to_owned(),
            category: EquipmentJsonKey::Chest,
            stats: crate::character_mod::stats::Stats::default(),
        };
        inventory.add_equipment(&equipment1, true);
        inventory.add_equipment(&equipment2, false);
        let equipments = inventory.get_all_equipments(
            vec![equipment1.clone(), equipment2.clone()].as_slice(),
            false,
        );
        assert_eq!(equipments.len(), 13);
        equipments.iter().for_each(|(category, equipments)| {
            if category == &EquipmentJsonKey::LeftWeapon.to_string() {
                assert_eq!(equipments.len(), 1);
                assert_eq!(equipments[0], equipment1);
            } else if category == &EquipmentJsonKey::Chest.to_string() {
                assert_eq!(equipments.len(), 1);
                assert_eq!(equipments[0], equipment2);
            } else {
                assert!(equipments.is_empty());
            }
        });

        // test get equipped equipments
        let equipped_equipments = inventory
            .get_equipped_equipments(vec![equipment1.clone(), equipment2.clone()].as_slice());
        assert_eq!(equipped_equipments.len(), 13);
        equipped_equipments
            .iter()
            .for_each(|(category, equipments)| {
                if category == &EquipmentJsonKey::LeftWeapon.to_string() {
                    assert_eq!(equipments.len(), 1);
                    assert_eq!(equipments[0], equipment1);
                } else {
                    assert!(equipments.is_empty());
                }
            });

        inventory.remove_equipment("sword_of_testing");
        let equipments = inventory.get_all_equipments(vec![equipment1.clone()].as_slice(), false);
        assert_eq!(equipments.len(), 13);
        equipments.iter().for_each(|(_category, equipments)| {
            assert!(equipments.is_empty());
        });
    }

    #[test]
    fn unit_sum_all_equipped_equipment_stat() {
        let mut inventory = Inventory::default();
        let mut equipment1 = Equipment {
            name: "Helmet of Testing".to_owned(),
            unique_name: "helmet_of_testing".to_owned(),
            category: EquipmentJsonKey::Head,
            stats: crate::character_mod::stats::Stats::default(),
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
        };
        equipment2.stats.all_stats.insert(
            HP.to_owned(),
            crate::character_mod::stats::Attribute {
                buf_equip_value: 20,
                buf_equip_percent: 20,
                ..Default::default()
            },
        );
        inventory.add_equipment(&equipment1, true);
        inventory.add_equipment(&equipment2, true);
        assert_eq!(
            inventory
                .sum_all_equipped_equipment_stat(HP, &vec![equipment1.clone(), equipment2.clone()]),
            (30, 30)
        );

        // test with character
        let (gm, _hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();
        gm.pm
            .current_player
            .inventory
            .sum_all_equipped_equipment_stat(
                PHYSICAL_POWER,
                &gm.pm
                    .equipment_table
                    .values()
                    .flatten()
                    .cloned()
                    .collect::<Vec<Equipment>>(),
            );
        assert_eq!(
            gm.pm
                .current_player
                .inventory
                .sum_all_equipped_equipment_stat(
                    PHYSICAL_POWER,
                    &gm.pm
                        .equipment_table
                        .values()
                        .flatten()
                        .cloned()
                        .collect::<Vec<Equipment>>()
                ),
            (30, 0)
        );
    }

    #[test]
    fn unit_get_equipment_by_name() {
        let mut inventory = Inventory::default();
        let equipment1 = Equipment {
            name: "Amulet of Testing".to_owned(),
            unique_name: "Amulet".to_owned(),
            category: EquipmentJsonKey::Amulet,
            stats: crate::character_mod::stats::Stats::default(),
        };
        inventory.add_equipment(&equipment1, true);
        let all_equipments = vec![equipment1.clone()];
        let found_equipment = inventory.get_equipment_by_name("Amulet", &all_equipments);
        assert!(found_equipment.is_some());
        assert_eq!(found_equipment.unwrap(), equipment1);

        let found_equipment = inventory.get_equipment_by_name("NonExisting", &all_equipments);
        assert!(found_equipment.is_none());
    }
}
