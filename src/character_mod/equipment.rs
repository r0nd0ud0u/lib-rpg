use std::{fmt, path::Path};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::{character_mod::stats::Stats, utils};
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

impl PartialOrd for EquipmentJsonKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EquipmentJsonKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare les noms des variantes sous forme de chaînes de caractères
        let self_str = self.to_string(); // "Weapon", "Armor", etc.
        let other_str = other.to_string();
        self_str.cmp(&other_str)
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
}

#[cfg(test)]
mod tests {
    use crate::common::constants::stats_const::*;

    use super::*;

    #[test]
    fn unit_try_new_from_json() {
        let file_path = "./tests/offlines/equipment/body/RightRing/starting_right_ring.json"; // Path to the JSON file
        let equipment = Equipment::try_new_from_json(file_path);
        assert!(equipment.is_ok());
        let equipment = equipment.unwrap();
        assert_eq!(equipment.name, "starting right ring");
        assert_eq!(equipment.category, EquipmentJsonKey::RightRing);
        assert_eq!(equipment.unique_name, "starting right ring");
        // stats
        // stats - aggro
        assert_eq!(0, equipment.stats.all_stats[AGGRO].buf_equip_percent);
        assert_eq!(0, equipment.stats.all_stats[AGGRO].buf_equip_value);
        // berserk rate
        assert_eq!(10, equipment.stats.all_stats[VIGOR].buf_equip_value);
        assert_eq!(0, equipment.stats.all_stats[VIGOR].buf_equip_percent);

        // wrong file
        let file_path = "./hehe.json"; // Path to the JSON file
        let equipment = Equipment::try_new_from_json(file_path);
        assert!(equipment.is_err());
    }
}
