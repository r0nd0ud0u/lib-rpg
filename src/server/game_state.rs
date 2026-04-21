use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    character_mod::stats_in_game::StatsInGame,
    server::{game_manager::ResultLaunchAttack, players_manager::GameAtkEffect},
};

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameStatus {
    #[default]
    StartGame = 0,
    StartRound,
    ValidateAction,
    EndOfGame,
    EndOfScenario,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultAtks {
    /// Number of auto attacks stored
    pub nb_atk_stored: i64,
    /// Effect outcomes of the auto attacks
    pub results: ResultLaunchAttack,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    /// Current turn number
    pub current_turn_nb: usize,
    /// Key turn number, value name
    pub died_ennemies: HashMap<usize, Vec<String>>,
    /// List in the ascending order of the players
    pub order_to_play: Vec<String>,
    /// Current round number
    pub current_round: usize,
    /// Name of the game
    pub game_name: String,
    /// Game Status
    pub status: GameStatus,
    /// Information about the last result attacks
    pub last_result_atk: ResultLaunchAttack,
    /// Stats in game, to display in the stats sheet
    pub stats_in_game: HashMap<String, StatsInGame>,
}

impl GameState {
    pub fn new() -> Self {
        // create name of exercise
        let time_str = crate::utils::get_current_time_as_string();
        GameState {
            game_name: format!("Game_{}", time_str),
            died_ennemies: HashMap::new(),
            order_to_play: Vec::new(),
            status: GameStatus::StartGame,
            ..Default::default()
        }
    }

    pub fn clear_scenario(&mut self) {
        self.current_turn_nb = 0;
        self.current_round = 0;
        self.died_ennemies.clear();
        self.order_to_play.clear();
        self.status = GameStatus::StartGame;
        self.last_result_atk = ResultLaunchAttack::default();
    }

    pub fn start_new_turn(&mut self) {
        // Increment turn number
        self.current_turn_nb += 1;
        // Reset to round 0
        self.current_round = 0;
    }

    pub fn new_round(&mut self) {
        self.current_round += 1;
    }

    pub fn process_game_stats(
        &mut self,
        new_gaes: &Vec<GameAtkEffect>,
        player_name: &str,
        atk_name: &str,
    ) {
        self.stats_in_game
            .entry(player_name.to_owned())
            .or_default()
            .process_all_game_stats(new_gaes, atk_name);
    }
}

#[cfg(test)]
mod tests {
    use crate::server::game_state::{GameState, GameStatus};

    #[test]
    fn unit_new() {
        let gs = GameState::new();
        assert!(gs.game_name.starts_with("Game_"));
        assert_eq!(gs.current_turn_nb, 0);
        assert_eq!(gs.current_round, 0);
        assert_eq!(gs.status, GameStatus::StartGame);
    }

    #[test]
    fn unit_start_new_turn() {
        let mut gs = GameState::new();
        gs.start_new_turn();
        assert_eq!(gs.current_round, 0);
        assert_eq!(gs.current_turn_nb, 1);
    }
}
