use crate::{common::paths_const::OFFLINE_CHARACTERS, players_manager::PlayerManager};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameManager {
    pub player_manager: PlayerManager,
}

impl GameManager {
    pub fn try_new() -> Result<GameManager> {
        let pm = PlayerManager::try_new(OFFLINE_CHARACTERS.as_os_str())?;
        Ok(GameManager { player_manager: pm })
    }
}

#[cfg(test)]
mod tests {
    use crate::game_manager::GameManager;

    #[test]
    fn unit_try_new() {
        let gm = GameManager::try_new().unwrap();
        assert_eq!(1, gm.player_manager.all_heroes.len());
    }
}
