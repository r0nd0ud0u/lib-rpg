use std::path::Path;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::{stats::Stats, utils};

/// Define the parameters of an equipment.
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Equipment {
    /// Name of the equipment
    #[serde(rename = "Categorie")]
    pub category: String,
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
pub struct EquipmentOnCharacter {
    #[serde(rename = "Head")]
    pub head: String,
    #[serde(rename = "Chest")]
    pub chest: String,
    #[serde(rename = "Shoes")]
    pub shoes: String,
    #[serde(rename = "Left Ring")]
    pub left_ring: String,
    #[serde(rename = "Right Ring")]
    pub right_ring: String,
    #[serde(rename = "Left Weapon")]
    pub left_weapon: String,
    #[serde(rename = "Right Weapon")]
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
    #[serde(rename = "Name")]
    pub name: String,
}

impl Equipment {
    pub fn try_new_from_json<P: AsRef<Path>>(path: P) -> Result<Equipment> {
        if let Ok(mut value) = utils::read_from_json::<_, Equipment>(&path) {
            value.stats.init();
            Ok(value)
        } else {
            Err(anyhow!("Unknown file: {:?}", path.as_ref()))
        }
    }

    pub fn decode_characters_equipment<P: AsRef<Path>>(path: P) -> Result<EquipmentOnCharacter> {
        utils::read_from_json::<_, EquipmentOnCharacter>(&path)
            .map_err(|_| anyhow!("Unknown file: {:?}", path.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use crate::common::stats_const::*;

    use super::*;

    #[test]
    fn unit_try_new_from_json() {
        let file_path =
            "./tests/offlines/equipment/body/right-ring/Anneau de Boromir-4-2024-05-11-12-36-16.json"; // Path to the JSON file
        let equipment = Equipment::try_new_from_json(file_path);
        assert!(equipment.is_ok());
        let equipment = equipment.unwrap();
        assert_eq!(equipment.name, "Anneau de Boromir");
        assert_eq!(equipment.category, "Right Ring");
        assert_eq!(
            equipment.unique_name,
            "Anneau de Boromir-4-2024-05-11-12-36-16"
        );
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
        let file_path = "./tests/offlines/equipment/characters/Test.json"; // Path to the JSON file
        let decoded_equipment = Equipment::decode_characters_equipment(file_path);
        assert!(decoded_equipment.is_ok());
        let decoded_equipment = decoded_equipment.unwrap();
        assert_eq!(decoded_equipment.head, "head");
        assert_eq!(decoded_equipment.chest, "chest");
        assert_eq!(decoded_equipment.shoes, "shoes");
        assert_eq!(decoded_equipment.left_ring, "left_ring");
        assert_eq!(decoded_equipment.right_ring, "right_ring");
        assert_eq!(decoded_equipment.left_weapon, "left_weapon");
        assert_eq!(decoded_equipment.right_weapon, "right_weapon");
        assert_eq!(decoded_equipment.amulet, "amulet");
        assert_eq!(decoded_equipment.belt, "belt");
        assert_eq!(decoded_equipment.cape, "cape");
        assert_eq!(decoded_equipment.pants, "pants");
        assert_eq!(decoded_equipment.tattoes, vec!["tattoo1", "tattoo2"]);
        assert_eq!(decoded_equipment.gloves, "gloves");
        assert_eq!(decoded_equipment.name, "name");
    }
}
