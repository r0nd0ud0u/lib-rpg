use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::{attack_type::AttackType, common::reach_const::INDIVIDUAL};

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameStatus {
    #[default]
    StartGame = 0,
    StartRound,
    ValidateAction,
    EndOfGame,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    /// Current turn number
    pub current_turn_nb: usize,
    /// Key turn number, value name
    pub died_ennemies: HashMap<usize, String>,
    /// List in the ascending order of the players
    pub order_to_play: Vec<String>,
    /// Current round number
    pub current_round: usize,
    /// Name of the game
    pub game_name: String,
    /// Game Status
    pub status: GameStatus,
    /// Targeted list
    pub current_targeted_list: HashSet<String>,
    /// Current atk selected
    pub current_atk: AttackType,
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            died_ennemies: HashMap::new(),
            order_to_play: Vec::new(),
            status: GameStatus::StartGame,
            ..Default::default()
        }
    }
    pub fn init(&mut self) {
        // create name of exercise
        let time_str = crate::utils::get_current_time_as_string();
        self.game_name = format!("Game_{}", time_str);
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

    pub fn update_targeted_list(&mut self, target_name: &str) {
        if self.current_atk.reach == INDIVIDUAL {
            self.current_targeted_list.clear();
        }
        self.current_targeted_list.insert(target_name.to_string());
    }
}

#[cfg(test)]
mod tests {
    use crate::game_state::GameState;

    #[test]
    fn unit_start_game() {
        let mut gs = GameState::default();
        gs.init();
        assert!(!gs.game_name.is_empty());
    }

    #[test]
    fn unit_start_new_turn() {
        let mut gs = GameState::new();
        gs.start_new_turn();
        assert_eq!(gs.current_round, 0);
        assert_eq!(gs.current_turn_nb, 1);
    }
}
