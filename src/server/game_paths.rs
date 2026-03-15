use crate::common::constants::paths_const::*;
use std::path::{Path, PathBuf};

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GamePaths {
    /// Root path for the game, where all the different files will be stored
    pub input_data_root: PathBuf,
    /// Path where the characters of the game are stored
    pub input_data_characters: PathBuf,
    /// Path where the equipments of the game are stored
    pub input_data_equipments: PathBuf,
    /// Path where the loot of the game are stored
    pub output_loot: PathBuf,
    /// Path where the ongoing effects of the game are stored
    pub output_ongoing_effects: PathBuf,
    /// Path where the game state of the game is stored
    pub output_game_state: PathBuf,
    /// Path where the stats in game of the game are stored
    pub output_stats_in_game: PathBuf,
    /// Path where the different games are stored
    pub output_games_dir: PathBuf,
    /// Path where the current game is stored
    pub output_current_game_dir: PathBuf,
}

impl GamePaths {
    pub fn new<P: AsRef<Path>>(data_path: P, game_name: &str) -> GamePaths {
        // join GAMES_DIR with game_name to create the current game dir
        let output_dir = GAMES_DIR.to_path_buf().join(game_name);
        GamePaths {
            input_data_root: data_path.as_ref().to_path_buf(),
            output_games_dir: GAMES_DIR.to_path_buf(),
            output_current_game_dir: output_dir.clone(),
            input_data_characters: data_path.as_ref().join(OFFLINE_CHARACTERS.to_path_buf()),
            input_data_equipments: data_path.as_ref().join(OFFLINE_EQUIPMENT.to_path_buf()),
            output_game_state: output_dir.join(OFFLINE_GAMESTATE.to_path_buf()),
            output_loot: output_dir.join(OFFLINE_LOOT_EQUIPMENT.to_path_buf()),
            output_ongoing_effects: output_dir.join(OFFLINE_EFFECTS.to_path_buf()),
            output_stats_in_game: output_dir.join(GAME_STATE_STATS_IN_GAME.to_path_buf()),
        }
    }
}
