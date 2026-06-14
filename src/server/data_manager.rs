use std::{collections::HashMap, path::Path};

use anyhow::{Result, bail};
use strum::IntoEnumIterator;

use crate::{
    character_mod::{
        character::{Character, CharacterKind},
        equipment::{Equipment, EquipmentJsonKey},
    },
    common::constants::paths_const::{
        OFFLINE_CHARACTERS, OFFLINE_LOOT_EQUIPMENT, OFFLINE_ROOT, OFFLINE_SCENARIOS,
    },
    server::scenario::Scenario,
    utils::list_files_in_dir,
};

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DataManager {
    /// List of all playable heroes -> player
    pub all_heroes: Vec<Character>,
    /// List of all playable bosses -> computer
    pub all_bosses: Vec<Character>,
    /// List of all scenarios
    pub all_scenarios: Vec<Scenario>,
    /// Equipment table mapping character names to their equipped items
    pub equipment_table: HashMap<EquipmentJsonKey, Vec<Equipment>>,
    /// Root path for offline files
    pub offline_root: std::path::PathBuf,
}

impl DataManager {
    /// Create a new game manager with the given path for the offline files and the default active characters
    pub fn try_new<P: AsRef<Path>>(path: P) -> Result<DataManager> {
        let mut dm = DataManager::default();

        // set offline root path
        let mut path_ref = path.as_ref();
        if path_ref.as_os_str().is_empty() {
            path_ref = &OFFLINE_ROOT;
        }
        dm.offline_root = path_ref.to_path_buf();

        // load all the equipments
        // must be loaded before loading the characters
        dm.load_all_equipments(path_ref)?;
        // load all the characters
        dm.load_all_characters(path_ref)?;
        // load all the scenarios
        dm.load_all_scenarios(path_ref)?;

        Ok(DataManager {
            all_heroes: dm.all_heroes,
            all_bosses: dm.all_bosses,
            all_scenarios: dm.all_scenarios,
            equipment_table: dm.equipment_table,
            offline_root: dm.offline_root,
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
    /// Load all the JSON files in a path `P` which corresponds to a directory.
    /// Characters are inserted in Hero or Boss lists.
    /// Sub-directories are treated as universe names (each file inside gets `.universe` set).
    pub fn load_all_characters<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        if path.as_ref().as_os_str().is_empty() {
            bail!("no root path")
        }
        let character_dir_path = path.as_ref().join(*OFFLINE_CHARACTERS);

        // Load top-level JSON files (default universe / no universe)
        if let Ok(list) = list_files_in_dir(&character_dir_path) {
            for character_path in &list {
                match Character::try_new_from_json(
                    character_path,
                    path.as_ref(),
                    false,
                    &self.equipment_table,
                ) {
                    Ok(c) => {
                        if c.kind == CharacterKind::Hero {
                            self.all_heroes.push(c);
                        } else {
                            self.all_bosses.push(c);
                        }
                    }
                    Err(e) => tracing::error!("{:?} cannot be decoded: {}", character_path, e),
                }
            }
        }

        // Load characters from sub-directories — each sub-dir is a universe
        if let Ok(universe_dirs) = crate::utils::list_dirs_in_dir(&character_dir_path) {
            for universe_dir in &universe_dirs {
                let universe_name = universe_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                match list_files_in_dir(universe_dir) {
                    Ok(list) => {
                        for character_path in &list {
                            match Character::try_new_from_json(
                                character_path,
                                path.as_ref(),
                                false,
                                &self.equipment_table,
                            ) {
                                Ok(mut c) => {
                                    if c.universe.is_empty() {
                                        c.universe = universe_name.clone();
                                    }
                                    if c.kind == CharacterKind::Hero {
                                        self.all_heroes.push(c);
                                    } else {
                                        self.all_bosses.push(c);
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("{:?} cannot be decoded: {}", character_path, e)
                                }
                            }
                        }
                    }
                    Err(e) => tracing::warn!(
                        "Cannot list files in universe dir {:?}: {}",
                        universe_dir,
                        e
                    ),
                }
            }
        }

        if self.all_heroes.is_empty() && self.all_bosses.is_empty() {
            tracing::warn!("No characters found in {:?}", character_dir_path);
        }

        Ok(())
    }

    pub fn load_all_scenarios<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        if path.as_ref().as_os_str().is_empty() {
            bail!("no root path")
        }
        let scenario_dir_path = path.as_ref().join(*OFFLINE_SCENARIOS);

        // Load top-level JSON files (default universe / no universe)
        if let Ok(list) = list_files_in_dir(&scenario_dir_path) {
            for scenario_path in &list {
                match Scenario::try_new_from_json(scenario_path) {
                    Ok(s) => self.all_scenarios.push(s),
                    Err(e) => tracing::error!("{:?} cannot be decoded: {}", scenario_path, e),
                }
            }
        }

        // Load scenarios from sub-directories — each sub-dir is a universe
        if let Ok(universe_dirs) = crate::utils::list_dirs_in_dir(&scenario_dir_path) {
            for universe_dir in &universe_dirs {
                let universe_name = universe_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                match list_files_in_dir(universe_dir) {
                    Ok(list) => {
                        for scenario_path in &list {
                            match Scenario::try_new_from_json(scenario_path) {
                                Ok(mut s) => {
                                    if s.universe.is_empty() {
                                        s.universe = universe_name.clone();
                                    }
                                    self.all_scenarios.push(s);
                                }
                                Err(e) => {
                                    tracing::error!("{:?} cannot be decoded: {}", scenario_path, e)
                                }
                            }
                        }
                    }
                    Err(e) => tracing::warn!(
                        "Cannot list files in universe dir {:?}: {}",
                        universe_dir,
                        e
                    ),
                }
            }
        }

        if self.all_scenarios.is_empty() {
            tracing::warn!("No scenarios found in {:?}", scenario_dir_path);
        }

        Ok(())
    }

    /// Return a sorted list of all distinct universes found in loaded scenarios.
    /// An empty string means the default universe (scenarios stored at the top level).
    pub fn list_universes(&self) -> Vec<String> {
        let mut universes: Vec<String> = self
            .all_scenarios
            .iter()
            .map(|s| s.universe.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        universes.sort();
        universes
    }

    /// Return all scenarios belonging to `universe`.
    /// Pass an empty string to get scenarios with no universe set.
    pub fn scenarios_by_universe(&self, universe: &str) -> Vec<&Scenario> {
        self.all_scenarios
            .iter()
            .filter(|s| s.universe == universe)
            .collect()
    }

    /// Return a sorted list of all distinct universes found in loaded heroes.
    pub fn list_hero_universes(&self) -> Vec<String> {
        let mut universes: Vec<String> = self
            .all_heroes
            .iter()
            .map(|c| c.universe.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        universes.sort();
        universes
    }

    /// Return all hero characters belonging to `universe`.
    pub fn heroes_by_universe(&self, universe: &str) -> Vec<&Character> {
        self.all_heroes
            .iter()
            .filter(|c| c.universe == universe)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use crate::{
        character_mod::equipment::EquipmentJsonKey,
        common::constants::paths_const::TEST_OFFLINE_ROOT, server::data_manager::DataManager,
        testing::testing_all_characters::testing_dm,
    };

    #[test]
    fn unit_try_new() {
        // offline_root with 2 heroes and 2 bosses (test data is unchanged)
        let dm = testing_dm();
        assert_eq!(dm.all_heroes.len(), 2);
        assert_eq!(dm.all_bosses.len(), 2);

        // offline_root by default (production data):
        // lotr: 4 heroes + 8 bosses; pokemon: 3 heroes + 9 bosses
        let dm = DataManager::try_new("").unwrap();
        assert_eq!(dm.all_heroes.len(), 7, "4 lotr heroes + 3 pokemon heroes");
        assert!(dm.all_bosses.len() >= 2, "at least the original 2 bosses");

        // offline_root by default with unknown file
        assert!(DataManager::try_new("unknown").is_err());

        // offline_root by default (again, consistent)
        let dm = DataManager::try_new("").unwrap();
        assert_eq!(dm.all_heroes.len(), 7);
    }

    #[test]
    fn unit_load_all_equipments() {
        let mut dm = DataManager::default();
        dm.load_all_equipments(*TEST_OFFLINE_ROOT).unwrap();
        assert_eq!(EquipmentJsonKey::iter().count(), dm.equipment_table.len());
    }

    #[test]
    fn unit_load_all_equipments_err() {
        let mut dm = DataManager::default();
        assert!(dm.load_all_equipments("").is_err());
    }

    #[test]
    fn unit_load_all_scenarios_err() {
        let mut dm = DataManager::default();
        assert!(dm.load_all_scenarios("").is_err());
    }

    #[test]
    fn unit_list_universes() {
        let dm = DataManager::try_new(*TEST_OFFLINE_ROOT).unwrap();
        let universes = dm.list_universes();
        assert!(universes.contains(&"".to_owned()), "default universe present");
    }

    #[test]
    fn unit_scenarios_by_universe() {
        let dm = DataManager::try_new(*TEST_OFFLINE_ROOT).unwrap();
        let default_scenarios = dm.scenarios_by_universe("");
        assert_eq!(default_scenarios.len(), 2);
        assert!(dm.scenarios_by_universe("nonexistent").is_empty());
    }

    #[test]
    fn unit_load_all_characters() {
        let mut dm = DataManager::default();
        dm.load_all_characters(*TEST_OFFLINE_ROOT).unwrap();
        assert_eq!(2, dm.all_heroes.len());
    }

    #[test]
    fn unit_load_all_characters_err() {
        let mut dm = DataManager::default();
        assert!(dm.load_all_characters("").is_err());
    }

    #[test]
    fn unit_load_all_scenarios() {
        let mut dm = DataManager::default();
        dm.load_all_scenarios(*TEST_OFFLINE_ROOT).unwrap();
        assert_eq!(2, dm.all_scenarios.len());
        // check the content of the first scenario
        // check if stage 1 is correctly loaded
        let stage_1 = dm
            .all_scenarios
            .iter()
            .find(|s| s.name == "Stage 1")
            .unwrap();
        assert_eq!(stage_1.description, "This is a test scenario");
        assert_eq!(stage_1.boss_patterns.len(), 1);
        assert_eq!(stage_1.boss_patterns["test_boss1"], vec![0]);
        assert_eq!(stage_1.loots.len(), 3);
        // stage 2 is correctly loaded
        let stage_2 = dm
            .all_scenarios
            .iter()
            .find(|s| s.name == "Stage 2")
            .unwrap();
        assert_eq!(stage_2.description, "The second stage of the game");
        assert_eq!(stage_2.boss_patterns.len(), 2);
        assert_eq!(stage_2.boss_patterns["test_boss1"], vec![0]);
        assert_eq!(stage_2.boss_patterns["test_boss2"], vec![0]);
        assert!(stage_2.loots.is_empty());
    }

    #[test]
    fn unit_heroes_by_universe_empty_default() {
        let dm = DataManager::default();
        assert!(dm.list_hero_universes().is_empty());
        assert!(dm.heroes_by_universe("lotr").is_empty());
    }

    #[test]
    fn unit_heroes_by_universe_production() {
        use crate::common::constants::paths_const::OFFLINE_ROOT;
        let mut dm = DataManager::default();
        dm.load_all_equipments(&*OFFLINE_ROOT).unwrap();
        dm.load_all_characters(&*OFFLINE_ROOT).unwrap();
        let universes = dm.list_hero_universes();
        assert!(!universes.is_empty(), "expected at least one universe");
        assert!(
            universes.contains(&"lotr".to_owned()),
            "expected 'lotr' universe"
        );
        let lotr_heroes = dm.heroes_by_universe("lotr");
        assert_eq!(lotr_heroes.len(), 4, "expected 4 lotr heroes");
        for hero in &lotr_heroes {
            assert_eq!(hero.universe, "lotr");
        }
    }

    #[test]
    fn unit_pokemon_heroes() {
        use crate::common::constants::paths_const::OFFLINE_ROOT;
        let mut dm = DataManager::default();
        dm.load_all_equipments(&*OFFLINE_ROOT).unwrap();
        dm.load_all_characters(&*OFFLINE_ROOT).unwrap();

        let universes = dm.list_hero_universes();
        assert!(
            universes.contains(&"pokemon".to_owned()),
            "expected 'pokemon' universe to be loaded"
        );

        let pokemon_heroes = dm.heroes_by_universe("pokemon");
        assert_eq!(
            pokemon_heroes.len(),
            3,
            "expected 3 pokemon heroes (Bulbasaur, Charmander, Squirtle)"
        );
        for hero in &pokemon_heroes {
            assert_eq!(hero.universe, "pokemon");
        }
        let names: Vec<&str> = pokemon_heroes
            .iter()
            .map(|h| h.db_full_name.as_str())
            .collect();
        assert!(names.contains(&"Bulbasaur"), "Bulbasaur should be loaded");
        assert!(names.contains(&"Charmander"), "Charmander should be loaded");
        assert!(names.contains(&"Squirtle"), "Squirtle should be loaded");
    }

    #[test]
    fn unit_pokemon_scenarios() {
        use crate::common::constants::paths_const::OFFLINE_ROOT;
        let mut dm = DataManager::default();
        dm.load_all_scenarios(&*OFFLINE_ROOT).unwrap();

        let pokemon_scenarios = dm.scenarios_by_universe("pokemon");
        assert_eq!(
            pokemon_scenarios.len(),
            10,
            "expected 10 pokemon scenarios (stage_1 through stage_10)"
        );
        for s in &pokemon_scenarios {
            assert_eq!(s.universe, "pokemon");
            assert!(!s.name.is_empty(), "scenario name should not be empty");
            assert!(
                !s.description.is_empty(),
                "scenario description should not be empty"
            );
        }
        let levels: Vec<u64> = pokemon_scenarios.iter().map(|s| s.level).collect();
        for i in 1..=10u64 {
            assert!(
                levels.contains(&i),
                "pokemon should have level {i} scenario"
            );
        }
    }

    #[test]
    fn unit_lotr_scenarios() {
        use crate::common::constants::paths_const::OFFLINE_ROOT;
        let mut dm = DataManager::default();
        dm.load_all_scenarios(&*OFFLINE_ROOT).unwrap();

        let lotr_scenarios = dm.scenarios_by_universe("lotr");
        assert_eq!(lotr_scenarios.len(), 10, "expected 10 lotr scenarios");
        for s in &lotr_scenarios {
            assert_eq!(s.universe, "lotr");
            assert!(!s.name.is_empty(), "lotr scenario name should not be empty");
        }
    }
}
