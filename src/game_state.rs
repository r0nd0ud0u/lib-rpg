use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

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
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            died_ennemies: HashMap::new(),
            order_to_play: Vec::new(),
            ..Default::default()
        }
    }
    pub fn init(&mut self) {
        // create name of exercise
        let time_str = crate::utils::get_current_time_as_string();
        self.game_name = format!("Game_{}", time_str);
    }
    pub fn start_new_turn(&mut self) -> Result<()> {
        // Increment turn number
        self.current_turn_nb += 1;
        // Reset to round 0
        self.current_round = 0;

        self.new_round();

        Ok(())
    }

    pub fn new_round(&mut self) {
        self.current_round += 1;
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
        assert!(gs.start_new_turn().is_ok());
        assert_eq!(gs.current_round, 1);
        assert_eq!(gs.current_turn_nb, 1);
    }
}
