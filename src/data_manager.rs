use std::{collections::HashMap, path::Path};

use anyhow::{Result, bail};
use strum::IntoEnumIterator;

use crate::{
    character::{Character, CharacterType},
    common::paths_const::{OFFLINE_CHARACTERS, OFFLINE_LOOT_EQUIPMENT, OFFLINE_ROOT},
    equipment::{Equipment, EquipmentJsonKey},
    utils::list_files_in_dir,
};

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DataManager {
    /// List of all playable heroes -> player
    pub all_heroes: Vec<Character>,
    /// List of all playable bosses -> computer
    pub all_bosses: Vec<Character>,
    /// Equipment table mapping character names to their equipped items
    pub equipment_table: HashMap<EquipmentJsonKey, Vec<Equipment>>,
}

impl DataManager {
    /// Create a new game manager with the given path for the offline files and the default active characters
    pub fn try_new<P: AsRef<Path>>(path: P) -> Result<DataManager> {
        let mut dm = DataManager::default();

        let mut new_path = path.as_ref();
        if new_path.as_os_str().is_empty() {
            new_path = &OFFLINE_ROOT;
        }
        // load all the equipments
        // must be loaded before loading the characters
        dm.load_all_equipments(new_path)?;
        // load all the characters
        dm.load_all_characters(new_path)?;

        Ok(DataManager {
            all_heroes: dm.all_heroes,
            all_bosses: dm.all_bosses,
            equipment_table: dm.equipment_table,
        })
    }

    pub fn load_all_equipments<P: AsRef<Path>>(&mut self, root_path: P) -> Result<()> {
        if root_path.as_ref().as_os_str().is_empty() {
            bail!("no root path")
        }
        let equipment_dir_path = root_path.as_ref().join(*OFFLINE_LOOT_EQUIPMENT);
        // for each part of the equipment, load all the equipments and insert them in the equipment table
        for part in EquipmentJsonKey::iter() {
            let part_dir_path = equipment_dir_path.join(part.to_string());
            match list_files_in_dir(&part_dir_path) {
                Ok(list) => {
                    list.iter().for_each(|equipment_path| {
                        match Equipment::try_new_from_json(equipment_path) {
                            Ok(e) => {
                                self.equipment_table
                                    .entry(part.clone())
                                    .or_default()
                                    .push(e);
                            }
                            Err(e) => {
                                tracing::error!("{:?} cannot be decoded: {}", equipment_path, e)
                            }
                        }
                    })
                }
                Err(e) => bail!("Files cannot be listed in {:#?}: {}", part_dir_path, e),
            };
        }
        Ok(())
    }

    /// Load all the JSON files in a path `P` which corresponds to a directory.
    /// Characters are inserted in Hero or Boss lists.
    pub fn load_all_characters<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        if path.as_ref().as_os_str().is_empty() {
            bail!("no root path")
        }
        let character_dir_path = path.as_ref().join(*OFFLINE_CHARACTERS);
        match list_files_in_dir(&character_dir_path) {
            Ok(list) => list.iter().for_each(|character_path| {
                match Character::try_new_from_json(
                    character_path,
                    path.as_ref(),
                    false,
                    &self.equipment_table,
                ) {
                    Ok(c) => {
                        if c.kind == CharacterType::Hero {
                            self.all_heroes.push(c);
                        } else {
                            self.all_bosses.push(c);
                        }
                    }
                    Err(e) => tracing::error!("{:?} cannot be decoded: {}", character_path, e),
                }
            }),
            Err(e) => bail!("Files cannot be listed in {:#?}: {}", character_dir_path, e),
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use crate::{
        data_manager::DataManager, equipment::EquipmentJsonKey, testing_all_characters::testing_dm,
    };

    #[test]
    fn unit_try_new() {
        // offline_root with 2 heroes and 2 bosses
        let dm = testing_dm();
        assert_eq!(dm.all_heroes.len(), 2);
        assert_eq!(dm.all_bosses.len(), 2);

        // offline_root by default but no file
        let dm = DataManager::try_new("").unwrap();
        assert_eq!(dm.all_heroes.len(), 4);
        assert_eq!(dm.all_bosses.len(), 2);

        // offline_root by default with unknown file
        assert!(DataManager::try_new("unknown").is_err());

        // offline_root by default
        let dm = DataManager::try_new("").unwrap();
        assert_eq!(dm.all_heroes.len(), 4);
        assert_eq!(dm.all_bosses.len(), 2);

        // offline_root by default but no file
        let dm = DataManager::try_new("").unwrap();
        assert_eq!(dm.all_heroes.len(), 4);
        assert_eq!(dm.all_bosses.len(), 2);
    }

    #[test]
    fn unit_load_all_equipments() {
        let mut dm = DataManager::default();
        dm.load_all_equipments("./tests/offlines").unwrap();
        assert_eq!(EquipmentJsonKey::iter().count(), dm.equipment_table.len());
    }

    #[test]
    fn unit_load_all_characters() {
        let mut dm = DataManager::default();
        dm.load_all_characters("tests/offlines").unwrap();
        assert_eq!(2, dm.all_heroes.len());
    }

    #[test]
    fn unit_load_all_characters_err() {
        let mut dm = DataManager::default();
        assert!(dm.load_all_characters("").is_err());
    }
}
