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
    pub game_manager: GameManager,
    /// TODO use in ServerData struct
    pub server_name: String,
    pub game_phase: GamePhase,
    /// reload info: players_nb
    pub players_nb: i64,
    /// reload info: key: username, value: character-name
    pub heroes_chosen: HashMap<String, String>,
}

impl CoreGameData {
    pub fn new(dm: &DataManager, server_name: &str) -> CoreGameData {
        let mut gm = GameManager::new(&dm.offline_root, dm.equipment_table.clone());
        // set bosses
        dm.all_bosses.iter().for_each(|boss| {
            let mut boss_to_push = boss.clone();
            boss_to_push.id_name = format!(
                "{}_#{}",
                boss_to_push.db_full_name,
                1 + gm
                    .pm
                    .get_nb_of_active_bosses_by_name(&boss_to_push.db_full_name)
            );
            gm.pm.active_bosses.push(boss_to_push);
        });
        gm.pm.active_bosses = dm.all_bosses.clone();

        CoreGameData {
            game_manager: gm,
            server_name: server_name.to_string(),
            game_phase: GamePhase::Default,
            players_nb: 0,
            heroes_chosen: HashMap::new(),
        }
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

        assert_eq!(
            core_game_data.game_manager.pm.active_bosses.len(),
            dm.all_bosses.len()
        );
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
