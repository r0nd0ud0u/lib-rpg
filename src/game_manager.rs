use std::path::Path;

use crate::{common::paths_const::OFFLINE_CHARACTERS, players_manager::PlayerManager};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// The entry of the library.
/// That object should be called to access to all the different functionalities.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameManager {
    pub player_manager: PlayerManager,
}

impl GameManager {
    pub fn try_new<P: AsRef<Path>>(path: P) -> Result<GameManager> {
        let pm =
            PlayerManager::try_new(path.as_ref().join(OFFLINE_CHARACTERS.as_path()).as_os_str())?;
        Ok(GameManager { player_manager: pm })
    }
}

#[cfg(test)]
mod tests {
    use crate::game_manager::GameManager;

    #[test]
    fn unit_try_new() {
        let gm = GameManager::try_new("").unwrap();
        assert!(gm.player_manager.all_heroes.len() > 1);
    }
}
