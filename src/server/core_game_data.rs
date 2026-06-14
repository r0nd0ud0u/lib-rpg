use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

use crate::server::data_manager::DataManager;
use crate::server::game_manager::GameManager;
use crate::server::server_manager::GamePhase;

/// Game core state, stored on the server and sent to clients
/// Those data are necessary to run/load/replay a game
#[derive(Default, Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CoreGameData {
    /// game manager, contains all the data of the game, including players, bosses, scenarios, logs, etc.
    pub game_manager: GameManager,
    /// Name of the server, used to identify the game and for clients to connect to the right game
    pub server_name: String,
    /// current game phase, used to know what actions are allowed and what data to send to clients
    pub game_phase: GamePhase,
    /// reload info: players_nb
    pub players_nb: i64,
    /// reload info: key: username, value: character-name
    pub heroes_chosen: HashMap<String, String>,
    /// single-player mode: one real player controls all heroes
    #[serde(default)]
    pub is_single_player: bool,
    /// universe selected at lobby creation; empty = all universes
    #[serde(default)]
    pub universe: String,
    /// true when the game was restored from a save file (universe and scenarios are fixed)
    #[serde(default)]
    pub loaded_from_save: bool,
}

impl CoreGameData {
    pub fn new(dm: &DataManager, server_name: &str) -> Result<CoreGameData> {
        Self::new_with_scenarios(dm, server_name, dm.all_scenarios.clone())
    }

    /// Like `new`, but uses a custom set of scenarios instead of all scenarios in `dm`.
    pub fn new_with_scenarios(
        dm: &DataManager,
        server_name: &str,
        scenarios: Vec<crate::server::scenario::Scenario>,
    ) -> Result<CoreGameData> {
        let mut gm = GameManager::new(&dm.offline_root, dm.equipment_table.clone(), scenarios);

        // set the full boss roster so load_next_scenario can populate active_bosses
        gm.pm.all_bosses = dm.all_bosses.clone();
        // load the first scenario of the game and set its active bosses
        gm.load_next_scenario()?;

        Ok(CoreGameData {
            game_manager: gm,
            server_name: server_name.to_owned(),
            game_phase: GamePhase::Default,
            players_nb: 0,
            heroes_chosen: HashMap::new(),
            is_single_player: false,
            universe: String::new(),
            loaded_from_save: false,
        })
    }

    pub fn load_next_scenario(&mut self) -> Result<()> {
        self.game_manager.load_next_scenario()
    }
}

#[cfg(test)]
mod tests {
    use crate::common::constants::paths_const::TEST_OFFLINE_ROOT;
    use crate::server::core_game_data::CoreGameData;
    use crate::server::data_manager::DataManager;

    #[test]
    fn unit_core_game_data_load_next_scenario() {
        let dm = DataManager::try_new(*TEST_OFFLINE_ROOT).unwrap();
        let mut core_game_data = CoreGameData::new(&dm, "Default").unwrap();
        let result = core_game_data.load_next_scenario();
        assert!(result.is_ok());
    }

    #[test]
    fn unit_core_game_data_new() {
        let dm = DataManager::try_new(*TEST_OFFLINE_ROOT).unwrap();
        let core_game_data = CoreGameData::new(&dm, "Default");

        assert!(core_game_data.is_ok());
        let core_game_data = core_game_data.unwrap();
        assert_eq!(core_game_data.game_manager.pm.active_bosses.len(), 1);
        // check that the id_name of the boss is correctly set
        for boss in &core_game_data.game_manager.pm.active_bosses {
            assert!(boss.id_name.starts_with(&boss.db_full_name));
            assert!(boss.id_name.ends_with("_#1"));
        }
        assert_eq!(core_game_data.server_name, "Default");
        assert_eq!(
            core_game_data.game_phase,
            crate::server::server_manager::GamePhase::Default
        );
        assert_eq!(core_game_data.players_nb, 0);
        assert!(core_game_data.heroes_chosen.is_empty());
        assert!(core_game_data.game_manager.logs.is_empty());
    }
}
