use std::{collections::HashMap, path::Path};

use anyhow::{Result, anyhow};
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
pub struct EquipmentOnCharacterJson {
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
    // TODO is it useful ?
    #[serde(rename = "Name")]
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum EquipmentOnCharacter {
    Head(String),
    Chest(String),
    Shoes(String),
    LeftRing(String),
    RightRing(String),
    LeftWeapon(String),
    RightWeapon(String),
    Amulet(String),
    Belt(String),
    Cape(String),
    Pants(String),
    Tattoes(Vec<String>),
    Gloves(String),
    // TODO is it useful ?
    Name(String),
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

    pub fn decode_characters_equipment<P: AsRef<Path>>(
        path: P,
    ) -> Result<HashMap<String, EquipmentOnCharacter>> {
        let e = utils::read_from_json::<_, EquipmentOnCharacterJson>(&path)
            .map_err(|_| anyhow!("Unknown file: {:?}", path.as_ref()));
        let mut equipment_on_character = HashMap::new();
        if let Ok(e) = e {
            equipment_on_character.insert("Head".to_string(), EquipmentOnCharacter::Head(e.head));
            equipment_on_character
                .insert("Chest".to_string(), EquipmentOnCharacter::Chest(e.chest));
            equipment_on_character
                .insert("Shoes".to_string(), EquipmentOnCharacter::Shoes(e.shoes));
            equipment_on_character.insert(
                "Left Ring".to_string(),
                EquipmentOnCharacter::LeftRing(e.left_ring),
            );
            equipment_on_character.insert(
                "Right Ring".to_string(),
                EquipmentOnCharacter::RightRing(e.right_ring),
            );
            equipment_on_character.insert(
                "Left Weapon".to_string(),
                EquipmentOnCharacter::LeftWeapon(e.left_weapon),
            );
            equipment_on_character.insert(
                "Right Weapon".to_string(),
                EquipmentOnCharacter::RightWeapon(e.right_weapon),
            );
            equipment_on_character
                .insert("Amulet".to_string(), EquipmentOnCharacter::Amulet(e.amulet));
            equipment_on_character.insert("Belt".to_string(), EquipmentOnCharacter::Belt(e.belt));
            equipment_on_character.insert("Cape".to_string(), EquipmentOnCharacter::Cape(e.cape));
            equipment_on_character
                .insert("Pants".to_string(), EquipmentOnCharacter::Pants(e.pants));
            equipment_on_character.insert(
                "Tattoes".to_string(),
                EquipmentOnCharacter::Tattoes(e.tattoes),
            );
            equipment_on_character
                .insert("Gloves".to_string(), EquipmentOnCharacter::Gloves(e.gloves));
            equipment_on_character.insert("Name".to_string(), EquipmentOnCharacter::Name(e.name));
        } else {
            tracing::error!(
                "Equipment for character cannot be decoded: {:?}",
                path.as_ref()
            );
        }
        Ok(equipment_on_character)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::stats_const::*;

    use super::*;

    #[test]
    fn unit_try_new_from_json() {
        let file_path = "./tests/offlines/equipment/body/right-ring/Anneau de Boromir-4-2024-05-11-12-36-16.json"; // Path to the JSON file
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
        assert_eq!(decoded_equipment.len(), 14);
        assert_eq!(
            decoded_equipment.get("Head").unwrap(),
            &EquipmentOnCharacter::Head("head".to_string())
        );
        assert_eq!(
            decoded_equipment.get("Chest").unwrap(),
            &EquipmentOnCharacter::Chest("chest".to_string())
        );
        assert_eq!(
            decoded_equipment.get("Shoes").unwrap(),
            &EquipmentOnCharacter::Shoes("shoes".to_string())
        );
        assert_eq!(
            decoded_equipment.get("Left Ring").unwrap(),
            &EquipmentOnCharacter::LeftRing("left_ring".to_string())
        );
        assert_eq!(
            decoded_equipment.get("Right Ring").unwrap(),
            &EquipmentOnCharacter::RightRing("right_ring".to_string())
        );
        assert_eq!(
            decoded_equipment.get("Left Weapon").unwrap(),
            &EquipmentOnCharacter::LeftWeapon("left_weapon".to_string())
        );
        assert_eq!(
            decoded_equipment.get("Right Weapon").unwrap(),
            &EquipmentOnCharacter::RightWeapon("right_weapon".to_string())
        );
        assert_eq!(
            decoded_equipment.get("Amulet").unwrap(),
            &EquipmentOnCharacter::Amulet("amulet".to_string())
        );
        assert_eq!(
            decoded_equipment.get("Belt").unwrap(),
            &EquipmentOnCharacter::Belt("belt".to_string())
        );
        assert_eq!(
            decoded_equipment.get("Cape").unwrap(),
            &EquipmentOnCharacter::Cape("cape".to_string())
        );
        assert_eq!(
            decoded_equipment.get("Pants").unwrap(),
            &EquipmentOnCharacter::Pants("pants".to_string())
        );
        assert_eq!(
            decoded_equipment.get("Tattoes").unwrap(),
            &EquipmentOnCharacter::Tattoes(vec!["tattoo1".to_string()])
        );
        assert_eq!(
            decoded_equipment.get("Gloves").unwrap(),
            &EquipmentOnCharacter::Gloves("gloves".to_string())
        );
        assert_eq!(
            decoded_equipment.get("Name").unwrap(),
            &EquipmentOnCharacter::Name("name".to_string())
        );
    }
}
