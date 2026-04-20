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
}

impl CoreGameData {
    pub fn new(dm: &DataManager, server_name: &str) -> Result<CoreGameData> {
        let mut gm = GameManager::new(
            &dm.offline_root,
            dm.equipment_table.clone(),
            dm.all_scenarios.clone(),
        );

        // load the first scenario of the game
        gm.load_next_scenario()?;

        // set active bosses
        gm.current_scenario
            .boss_patterns
            .iter()
            .for_each(|(boss_name, _)| {
                if let Some(b) = dm.all_bosses.iter().find(|b| b.db_full_name == *boss_name) {
                    let mut boss_to_push = b.clone();
                    boss_to_push.id_name = format!(
                        "{}_#{}",
                        boss_to_push.db_full_name,
                        1 + gm
                            .pm
                            .get_nb_of_active_bosses_by_name(&boss_to_push.db_full_name)
                    );
                    gm.pm.active_bosses.push(boss_to_push);
                } else {
                    tracing::warn!("Boss {} not found in data manager, skipping it", boss_name);
                }
            });

        Ok(CoreGameData {
            game_manager: gm,
            server_name: server_name.to_owned(),
            game_phase: GamePhase::Default,
            players_nb: 0,
            heroes_chosen: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::common::constants::paths_const::TEST_OFFLINE_ROOT;
    use crate::server::core_game_data::CoreGameData;
    use crate::server::data_manager::DataManager;

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
