use std::{collections::HashMap, fmt, path::Path};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::{stats::Stats, utils};
use strum_macros::EnumIter;

/// Define the parameters of an equipment.
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Equipment {
    /// Name of the equipment
    #[serde(rename = "Categorie")]
    pub category: EquipmentJsonKey,
    /// Type of the equipment
    #[serde(rename = "Nom")]
    pub name: String,
    /// Photo of the equipment
    #[serde(rename = "Nom unique")]
    pub unique_name: String,
    /// Stats of the equipment
    #[serde(rename = "Stats")]
    pub stats: Stats,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct EquipmentOnCharacterJson {
    #[serde(rename = "Head")]
    pub head: String,
    #[serde(rename = "Chest")]
    pub chest: String,
    #[serde(rename = "Shoes")]
    pub shoes: String,
    #[serde(rename = "LeftRing")]
    pub left_ring: String,
    #[serde(rename = "RightRing")]
    pub right_ring: String,
    #[serde(rename = "LeftWeapon")]
    pub left_weapon: String,
    #[serde(rename = "RightWeapon")]
    pub right_weapon: String,
    #[serde(rename = "Amulet")]
    pub amulet: String,
    #[serde(rename = "Belt")]
    pub belt: String,
    #[serde(rename = "Cape")]
    pub cape: String,
    #[serde(rename = "Pants")]
    pub pants: String,
    #[serde(default, rename = "Tattoes")]
    pub tattoes: Vec<String>,
    #[serde(rename = "Gloves")]
    pub gloves: String,
    // TODO is it useful ?
    #[serde(rename = "Name")]
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum EquipmentJsonValue {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Default, EnumIter)]
#[serde(rename_all = "PascalCase")]
pub enum EquipmentJsonKey {
    #[default]
    Head,
    Chest,
    Shoes,
    LeftRing,
    RightRing,
    LeftWeapon,
    RightWeapon,
    Amulet,
    Belt,
    Cape,
    Pants,
    Tattoes,
    Gloves,
}

impl fmt::Display for EquipmentJsonKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EquipmentJsonKey::Head => "Head",
            EquipmentJsonKey::Chest => "Chest",
            EquipmentJsonKey::Shoes => "Shoes",
            EquipmentJsonKey::LeftRing => "LeftRing",
            EquipmentJsonKey::RightRing => "RightRing",
            EquipmentJsonKey::LeftWeapon => "LeftWeapon",
            EquipmentJsonKey::RightWeapon => "RightWeapon",
            EquipmentJsonKey::Amulet => "Amulet",
            EquipmentJsonKey::Belt => "Belt",
            EquipmentJsonKey::Cape => "Cape",
            EquipmentJsonKey::Pants => "Pants",
            EquipmentJsonKey::Tattoes => "Tattoes",
            EquipmentJsonKey::Gloves => "Gloves",
        };
        write!(f, "{}", s)
    }
}

impl Equipment {
    pub fn try_new_from_json<P: AsRef<Path>>(path: P) -> Result<Equipment> {
        if let Ok(mut value) = utils::read_from_json::<_, Equipment>(&path) {
            value.stats.init();
            Ok(value)
        } else {
            // output the true error for debugging
            let error = utils::read_from_json::<_, Equipment>(&path).err();
            Err(anyhow!(
                "Error reading equipment from JSON: {:?}, error: {:?}",
                path.as_ref(),
                error
            ))
        }
    }

    pub fn decode_characters_equipment<P: AsRef<Path>>(
        path: P,
    ) -> Result<HashMap<EquipmentJsonKey, EquipmentJsonValue>> {
        let e = utils::read_from_json::<_, EquipmentOnCharacterJson>(&path)
            .map_err(|_| anyhow!("Unknown file: {:?}", path.as_ref()));
        let mut equipment_on_character = HashMap::new();
        if let Ok(e) = e {
            equipment_on_character
                .insert(EquipmentJsonKey::Head, EquipmentJsonValue::Single(e.head));
            equipment_on_character
                .insert(EquipmentJsonKey::Chest, EquipmentJsonValue::Single(e.chest));
            equipment_on_character
                .insert(EquipmentJsonKey::Shoes, EquipmentJsonValue::Single(e.shoes));
            equipment_on_character.insert(
                EquipmentJsonKey::LeftRing,
                EquipmentJsonValue::Single(e.left_ring),
            );
            equipment_on_character.insert(
                EquipmentJsonKey::RightRing,
                EquipmentJsonValue::Single(e.right_ring),
            );
            equipment_on_character.insert(
                EquipmentJsonKey::LeftWeapon,
                EquipmentJsonValue::Single(e.left_weapon),
            );
            equipment_on_character.insert(
                EquipmentJsonKey::RightWeapon,
                EquipmentJsonValue::Single(e.right_weapon),
            );
            equipment_on_character.insert(
                EquipmentJsonKey::Amulet,
                EquipmentJsonValue::Single(e.amulet),
            );
            equipment_on_character
                .insert(EquipmentJsonKey::Belt, EquipmentJsonValue::Single(e.belt));
            equipment_on_character
                .insert(EquipmentJsonKey::Cape, EquipmentJsonValue::Single(e.cape));
            equipment_on_character
                .insert(EquipmentJsonKey::Pants, EquipmentJsonValue::Single(e.pants));
            equipment_on_character.insert(
                EquipmentJsonKey::Tattoes,
                EquipmentJsonValue::Multiple(e.tattoes),
            );
            equipment_on_character.insert(
                EquipmentJsonKey::Gloves,
                EquipmentJsonValue::Single(e.gloves),
            );
        } else {
            return Err(anyhow!(
                "Equipment for character cannot be decoded: {:?}",
                path.as_ref()
            ));
        }
        Ok(equipment_on_character)
    }
}

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use crate::common::stats_const::*;

    use super::*;

    #[test]
    fn unit_try_new_from_json() {
        let file_path = "./tests/offlines/equipment/body/RightRing/right_ring_unique.json"; // Path to the JSON file
        let equipment = Equipment::try_new_from_json(file_path);
        assert!(equipment.is_ok());
        let equipment = equipment.unwrap();
        assert_eq!(equipment.name, "right_ring");
        assert_eq!(equipment.category, EquipmentJsonKey::RightRing);
        assert_eq!(equipment.unique_name, "right_ring_unique");
        // stats
        // stats - aggro
        assert_eq!(0, equipment.stats.all_stats[AGGRO].buf_equip_percent);
        assert_eq!(0, equipment.stats.all_stats[AGGRO].buf_equip_value);
        // berseck rate
        assert_eq!(4, equipment.stats.all_stats[BERSECK_RATE].buf_equip_value);
        assert_eq!(0, equipment.stats.all_stats[BERSECK_RATE].buf_equip_percent);

        // wrong file
        let file_path = "./hehe.json"; // Path to the JSON file
        let equipment = Equipment::try_new_from_json(file_path);
        assert!(equipment.is_err());
    }

    #[test]
    fn unit_decode_characters_equipment() {
        let file_path = "./tests/offlines/equipment/characters/test.json"; // Path to the JSON file
        let decoded_equipment = Equipment::decode_characters_equipment(file_path);
        assert!(decoded_equipment.is_ok());
        let decoded_equipment = decoded_equipment.unwrap();
        assert_eq!(decoded_equipment.len(), EquipmentJsonKey::iter().count());
        assert_eq!(
            decoded_equipment.get(&EquipmentJsonKey::Head).unwrap(),
            &EquipmentJsonValue::Single("head_unique".to_string())
        );
        assert_eq!(
            decoded_equipment.get(&EquipmentJsonKey::Chest).unwrap(),
            &EquipmentJsonValue::Single("chest_unique".to_string())
        );
        assert_eq!(
            decoded_equipment.get(&EquipmentJsonKey::Shoes).unwrap(),
            &EquipmentJsonValue::Single("shoes_unique".to_string())
        );
        assert_eq!(
            decoded_equipment.get(&EquipmentJsonKey::LeftRing).unwrap(),
            &EquipmentJsonValue::Single("left_ring_unique".to_string())
        );
        assert_eq!(
            decoded_equipment.get(&EquipmentJsonKey::RightRing).unwrap(),
            &EquipmentJsonValue::Single("right_ring_unique".to_string())
        );
        assert_eq!(
            decoded_equipment
                .get(&EquipmentJsonKey::LeftWeapon)
                .unwrap(),
            &EquipmentJsonValue::Single("left_weapon_unique".to_string())
        );
        assert_eq!(
            decoded_equipment
                .get(&EquipmentJsonKey::RightWeapon)
                .unwrap(),
            &EquipmentJsonValue::Single("right_weapon_unique".to_string())
        );
        assert_eq!(
            decoded_equipment.get(&EquipmentJsonKey::Amulet).unwrap(),
            &EquipmentJsonValue::Single("amulet_unique".to_string())
        );
        assert_eq!(
            decoded_equipment.get(&EquipmentJsonKey::Belt).unwrap(),
            &EquipmentJsonValue::Single("belt_unique".to_string())
        );
        assert_eq!(
            decoded_equipment.get(&EquipmentJsonKey::Cape).unwrap(),
            &EquipmentJsonValue::Single("cape_unique".to_string())
        );
        assert_eq!(
            decoded_equipment.get(&EquipmentJsonKey::Pants).unwrap(),
            &EquipmentJsonValue::Single("pants_unique".to_string())
        );
        assert_eq!(
            decoded_equipment.get(&EquipmentJsonKey::Tattoes).unwrap(),
            &EquipmentJsonValue::Multiple(vec![
                "tattoo1_unique".to_string(),
                "tattoo2_unique".to_string()
            ])
        );
        assert_eq!(
            decoded_equipment.get(&EquipmentJsonKey::Gloves).unwrap(),
            &EquipmentJsonValue::Single("gloves_unique".to_string())
        );
    }
}
