use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

use crate::data_manager::DataManager;
use crate::game_manager::GameManager;
use crate::game_manager::LogAtk;
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
    /// logs of the game, to display in the log sheet
    pub logs: Vec<LogAtk>,
}

impl CoreGameData {
    pub fn new(dm: &DataManager) -> CoreGameData {
        let mut gm = GameManager::new("offlines", dm.equipment_table.clone());
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
            server_name: "Default".to_owned(),
            game_phase: GamePhase::Default,
            players_nb: 0,
            heroes_chosen: HashMap::new(),
            logs: Vec::new(),
        }
    }
}

pub fn init(name: &str, core_game_data: &mut CoreGameData) {
    core_game_data.game_manager.init_new_game();
    // name of the server
    // TODO set server name based on user name + random string
    core_game_data.server_name = name.to_string();
}
