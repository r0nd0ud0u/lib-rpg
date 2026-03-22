use std::{collections::HashMap, path::PathBuf};

use crate::server::core_game_data::CoreGameData;

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ServerManager {
    /// key is player_name, value is a list of player_id (to handle multiple connections with the same player name, e.g. multiple tabs)
    pub players: HashMap<String, Vec<u32>>,
    /// List of paths to ongoing games, used to display on the load game page and to reconnect to ongoing games on server restart
    pub ongoing_games: Vec<OnGoingGame>,
    /// key is server_name, value is the server data (core game data and players data connected to the server)
    pub servers_data: HashMap<String, ServerData>,
    /// List of paths to saved games, used to display on the load game page
    pub saved_games_list: Vec<PathBuf>,
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct OnGoingGame {
    pub path: PathBuf,
    pub server_name: String,
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ServerData {
    // Necessary data to run the game on the server side and restart the game
    pub core_game_data: CoreGameData,
    // Processed data after start of the game
    pub players_data: PlayersData,
}

/// Data about the players connected to the server
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PlayersData {
    pub players_info: HashMap<String, PlayerInfo>,
    pub owner_player_name: String,
}

impl PlayersData {
    pub fn get_first_character_name(&self, player_name: &str) -> Option<String> {
        self.players_info
            .get(player_name)
            .and_then(|info| info.character_id_names.first().cloned())
    }
}

impl ServerData {
    pub fn reset(game_phase: GamePhase) -> ServerData {
        ServerData {
            core_game_data: CoreGameData {
                game_phase,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PlayerInfo {
    pub character_id_names: Vec<String>,
    pub player_ids: Vec<u32>,
}

impl ServerManager {
    pub fn new() -> Self {
        ServerManager {
            players: HashMap::new(),
            ongoing_games: Vec::new(),
            servers_data: HashMap::new(),
            saved_games_list: Vec::new(),
        }
    }

    pub fn add_player(&mut self, player_name: String, player_id: u32) {
        self.players
            .entry(player_name.clone())
            .or_default()
            .push(player_id);
    }

    pub fn add_server_data(
        &mut self,
        server_name: &str,
        core_game_data: &CoreGameData,
        player_name: &str,
    ) {
        self.servers_data.insert(
            server_name.to_string(),
            ServerData {
                core_game_data: core_game_data.clone(),
                players_data: PlayersData {
                    players_info: HashMap::new(),
                    owner_player_name: player_name.to_string(),
                },
            },
        );
    }

    pub fn add_player_to_server(&mut self, server_name: &str, player_name: &str, player_id: u32) {
        if let Some(server_data) = self.servers_data.get_mut(server_name) {
            server_data
                .players_data
                .players_info
                .entry(player_name.to_string())
                .or_default()
                .player_ids
                .push(player_id);
            if server_data.core_game_data.game_phase == GamePhase::InitGame {
                server_data.core_game_data.players_nb += 1;
            }
            if server_data.core_game_data.game_phase == GamePhase::Loading
                && let Some(character_name) =
                    server_data.core_game_data.heroes_chosen.get(player_name)
            {
                server_data
                    .players_data
                    .players_info
                    .entry(player_name.to_string())
                    .or_default()
                    .character_id_names
                    .push(character_name.clone());
            }
        }
    }

    /// Get the server data associated with a given player ID by searching through the servers data.
    pub fn get_server_data_by_player_id(&self, player_id: u32) -> Option<ServerData> {
        for server_data in self.servers_data.values() {
            for player_info in server_data.players_data.players_info.values() {
                if player_info.player_ids.contains(&player_id) {
                    return Some(server_data.clone());
                }
            }
        }
        None
    }
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum GamePhase {
    #[default]
    Default,
    InitGame,
    Loading,
    Running,
    Ended,
}

#[cfg(test)]
mod tests {
    use crate::server::server_manager::{PlayerInfo, PlayersData};

    #[test]
    fn test_get_first_character_name() {
        let mut players_data = PlayersData::default();
        players_data.players_info.insert(
            "player1".to_string(),
            PlayerInfo {
                character_id_names: vec!["character1".to_string(), "character2".to_string()],
                player_ids: vec![1, 2],
            },
        );

        assert_eq!(
            players_data.get_first_character_name("player1"),
            Some("character1".to_string())
        );
        assert_eq!(players_data.get_first_character_name("player2"), None);
    }
}
