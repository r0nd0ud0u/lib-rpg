use crate::players_manager::PlayerManager;

#[derive(Debug, Clone)]
pub struct GameManager {
    pub player_manager: PlayerManager,
}

impl GameManager {
    pub fn build() -> GameManager {
        GameManager {
            player_manager: PlayerManager::build(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::game_manager::GameManager;

    #[test]
    fn unit_build() {
       let gm = GameManager::build(); 
       assert_eq!(1, gm.player_manager.all_heroes.len());
    }
}
