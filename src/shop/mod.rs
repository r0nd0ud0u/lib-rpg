use std::collections::HashMap;

use crate::character_mod::effect::{build_energy_effect, build_hp_effect, build_resurrect_effect};
use crate::character_mod::inventory::ConsumableKind;
use crate::{
    character_mod::{
        equipment::{Equipment, EquipmentJsonKey},
        inventory::Consumable,
        loot::LootType,
        rank::Rank,
    },
    common::constants::stats_const::{BERSERK, MANA, VIGOR},
};

/// A single item available for purchase in the shop.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ShopCatalogItem {
    pub name: String,
    pub kind: LootType,
    pub price: u64,
    pub rank: Rank,
    /// `None` for consumables; set for equipment items.
    pub category: Option<EquipmentJsonKey>,
    pub description: String,
}

impl Default for ShopCatalogItem {
    fn default() -> Self {
        Self {
            name: String::new(),
            kind: LootType::Equipment,
            price: 0,
            rank: Rank::Common,
            category: None,
            description: String::new(),
        }
    }
}

/// Sell price is half the buy price.
pub fn sell_price(buy_price: u64) -> u64 {
    buy_price / 2
}

fn equipment_price(unique_name: &str) -> u64 {
    if unique_name.starts_with("starting") {
        100
    } else if unique_name.starts_with("medium") {
        300
    } else {
        200
    }
}

/// Build the shop catalog from loaded equipment and hardcoded consumables.
/// Tattoos are character-specific and are excluded.
pub fn build_shop_catalog(
    equipment_table: &HashMap<EquipmentJsonKey, Vec<Equipment>>,
) -> Vec<ShopCatalogItem> {
    let mut items: Vec<ShopCatalogItem> = Vec::new();

    // Equipment items (excluding Tattoes)
    for (category, equipments) in equipment_table {
        if *category == EquipmentJsonKey::Tattoes {
            continue;
        }
        for equip in equipments {
            let price = equipment_price(&equip.unique_name);
            // Collect non-zero stat bonuses for description
            let stat_lines: Vec<String> = equip
                .stats
                .all_stats
                .iter()
                .filter_map(|(k, v)| {
                    let has_value = v.buf_equip_value != 0;
                    let has_percent = v.buf_equip_percent != 0;
                    if has_value && has_percent {
                        Some(format!(
                            "{k}: +{} (+{}%)",
                            v.buf_equip_value, v.buf_equip_percent
                        ))
                    } else if has_value {
                        Some(format!("{k}: +{}", v.buf_equip_value))
                    } else if has_percent {
                        Some(format!("{k}: +{}%", v.buf_equip_percent))
                    } else {
                        None
                    }
                })
                .collect();
            let description = if stat_lines.is_empty() {
                "No stat bonuses.".to_owned()
            } else {
                stat_lines.join(", ")
            };
            items.push(ShopCatalogItem {
                name: equip.unique_name.clone(),
                kind: LootType::Equipment,
                price,
                rank: Rank::Common,
                category: Some(category.clone()),
                description,
            });
        }
    }

    // Hardcoded consumables (no JSON files for these)
    let consumables = [
        ("potion", 50u64, Rank::Common, "Restores 20 HP."),
        ("super potion", 150, Rank::Intermediate, "Restores 60 HP."),
        ("hyper potion", 300, Rank::Advanced, "Restores 120 HP."),
        (
            "potion of resurrection",
            500,
            Rank::Advanced,
            "Revives a fallen hero with 50 HP.",
        ),
        ("mana potion", 80, Rank::Common, "Restores 30 Mana."),
        ("vigor potion", 80, Rank::Common, "Restores 30 Vigor."),
        ("berserk potion", 80, Rank::Common, "Restores 30 Berserk."),
    ];
    for (name, price, rank, desc) in consumables {
        items.push(ShopCatalogItem {
            name: name.to_owned(),
            kind: LootType::Consumable,
            price,
            rank,
            category: None,
            description: desc.to_owned(),
        });
    }

    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

/// Build a `Consumable` from its shop name.
pub fn build_consumable_by_name(name: &str) -> Option<Consumable> {
    match name {
        "potion" => Some(Consumable {
            name: "potion".to_owned(),
            effects: vec![build_hp_effect(20, false)],
            consumable_kind: ConsumableKind::Potion,
            rank: Rank::Common,
        }),
        "super potion" => Some(Consumable {
            name: "super potion".to_owned(),
            effects: vec![build_hp_effect(60, false)],
            consumable_kind: ConsumableKind::Potion,
            rank: Rank::Intermediate,
        }),
        "hyper potion" => Some(Consumable {
            name: "hyper potion".to_owned(),
            effects: vec![build_hp_effect(120, false)],
            consumable_kind: ConsumableKind::Potion,
            rank: Rank::Advanced,
        }),
        "potion of resurrection" => Some(Consumable {
            name: "potion of resurrection".to_owned(),
            effects: vec![build_resurrect_effect(50)],
            consumable_kind: ConsumableKind::Potion,
            rank: Rank::Advanced,
        }),
        "mana potion" => Some(Consumable {
            name: "mana potion".to_owned(),
            effects: vec![build_energy_effect(MANA, 30)],
            consumable_kind: ConsumableKind::Potion,
            rank: Rank::Common,
        }),
        "vigor potion" => Some(Consumable {
            name: "vigor potion".to_owned(),
            effects: vec![build_energy_effect(VIGOR, 30)],
            consumable_kind: ConsumableKind::Potion,
            rank: Rank::Common,
        }),
        "berserk potion" => Some(Consumable {
            name: "berserk potion".to_owned(),
            effects: vec![build_energy_effect(BERSERK, 30)],
            consumable_kind: ConsumableKind::Potion,
            rank: Rank::Common,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        character_mod::equipment::EquipmentJsonKey, common::constants::paths_const::OFFLINE_ROOT,
        server::data_manager::DataManager,
    };

    fn catalog_from_production() -> Vec<ShopCatalogItem> {
        let mut dm = DataManager::default();
        dm.load_all_equipments(&*OFFLINE_ROOT).unwrap();
        build_shop_catalog(&dm.equipment_table)
    }

    #[test]
    fn unit_build_shop_catalog_excludes_tattoos() {
        let catalog = catalog_from_production();
        for item in &catalog {
            assert!(
                item.category != Some(EquipmentJsonKey::Tattoes),
                "Tattoos should not appear in catalog"
            );
        }
    }

    #[test]
    fn unit_build_shop_catalog_includes_consumables() {
        let catalog = catalog_from_production();
        let names: Vec<&str> = catalog.iter().map(|i| i.name.as_str()).collect();
        for expected in &[
            "potion",
            "super potion",
            "hyper potion",
            "potion of resurrection",
            "mana potion",
            "vigor potion",
            "berserk potion",
        ] {
            assert!(names.contains(expected), "Missing consumable: {expected}");
        }
    }

    #[test]
    fn unit_build_shop_catalog_price_tiers() {
        let catalog = catalog_from_production();
        for item in &catalog {
            if item.kind == LootType::Equipment {
                let name = &item.name;
                if name.starts_with("starting") {
                    assert_eq!(item.price, 100, "starting item should cost 100: {name}");
                } else if name.starts_with("medium") {
                    assert_eq!(item.price, 300, "medium item should cost 300: {name}");
                }
            }
        }
    }

    #[test]
    fn unit_sell_price_is_half() {
        assert_eq!(sell_price(100), 50);
        assert_eq!(sell_price(300), 150);
        assert_eq!(sell_price(1), 0);
    }

    #[test]
    fn unit_build_consumable_by_name_known() {
        assert!(build_consumable_by_name("potion").is_some());
        assert!(build_consumable_by_name("hyper potion").is_some());
        assert!(build_consumable_by_name("potion of resurrection").is_some());
    }

    #[test]
    fn unit_build_consumable_by_name_unknown() {
        assert!(build_consumable_by_name("unknown item").is_none());
    }
}
