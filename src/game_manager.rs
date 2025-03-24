use std::path::Path;

use crate::{
    common::paths_const::OFFLINE_CHARACTERS, game_state::GameState, players_manager::PlayerManager,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// The entry of the library.
/// That object should be called to access to all the different functionalities.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameManager {
    pub player_manager: PlayerManager,
    pub game_name: GameState,
}

impl GameManager {
    pub fn try_new<P: AsRef<Path>>(path: P) -> Result<GameManager> {
        let mut new_path = path.as_ref();
        if new_path.as_os_str().is_empty() {
            new_path = &OFFLINE_CHARACTERS;
        }
        let pm = PlayerManager::try_new(new_path)?;
        Ok(GameManager {
            player_manager: pm,
            game_name: GameState::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::game_manager::GameManager;

    #[test]
    fn unit_try_new() {
        let gm = GameManager::try_new("").unwrap();
        assert_eq!(gm.player_manager.all_heroes.len(), 0);

        let gm = GameManager::try_new("./tests/characters").unwrap();
        assert_eq!(gm.player_manager.all_heroes.len(), 1);
    }
}
