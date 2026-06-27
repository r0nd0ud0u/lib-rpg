use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

use crate::server::data_manager::DataManager;
use crate::server::game_manager::GameManager;
use crate::server::game_state::GameStatus;
use crate::server::overworld_manager::{OverworldManager, OverworldState};
use crate::server::server_manager::GamePhase;
use crate::shop::ShopCatalogItem;

/// Game core state, stored on the server and sent to clients
/// Those data are necessary to run/load/replay a game
#[derive(Default, Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CoreGameData {
    /// game manager, contains all the data of the game, including players, bosses, scenarios, logs, etc.
    pub game_manager: GameManager,
    /// Name of the server, used to identify the game and for clients to connect to the right game
    pub server_name: String,
    /// current game phase, used to know what actions are allowed and what data to send to clients
    pub game_phase: GamePhase,
    /// reload info: players_nb
    pub players_nb: i64,
    /// reload info: key: username, value: character-name
    pub heroes_chosen: HashMap<String, String>,
    /// single-player mode: one real player controls all heroes
    #[serde(default)]
    pub is_single_player: bool,
    /// universe selected at lobby creation; empty = all universes
    #[serde(default)]
    pub universe: String,
    /// true when the game was restored from a save file (universe and scenarios are fixed)
    #[serde(default)]
    pub loaded_from_save: bool,
    /// Shop catalog — items available for purchase and their prices
    #[serde(default)]
    pub shop_catalog: Vec<ShopCatalogItem>,
    /// Display hint for the action banner (consumable use, etc.).
    /// Empty after a real attack (banner reads from last_result_atk instead).
    #[serde(default)]
    pub last_action_header: String,
    /// Active overworld state; `Some` while `game_phase == Overworld`.
    #[serde(default)]
    pub overworld: Option<OverworldState>,
}

impl CoreGameData {
    pub fn new(dm: &DataManager, server_name: &str) -> Result<CoreGameData> {
        Self::new_with_scenarios(dm, server_name, dm.all_scenarios.clone())
    }

    /// Like `new`, but uses a custom set of scenarios instead of all scenarios in `dm`.
    pub fn new_with_scenarios(
        dm: &DataManager,
        server_name: &str,
        scenarios: Vec<crate::server::scenario::Scenario>,
    ) -> Result<CoreGameData> {
        let mut gm = GameManager::new(&dm.offline_root, dm.equipment_table.clone(), scenarios);

        // set the full boss roster so load_next_scenario can populate active_bosses
        gm.pm.all_bosses = dm.all_bosses.clone();
        // load the first scenario of the game and set its active bosses
        gm.load_next_scenario()?;

        Ok(CoreGameData {
            game_manager: gm,
            server_name: server_name.to_owned(),
            game_phase: GamePhase::Default,
            players_nb: 0,
            heroes_chosen: HashMap::new(),
            is_single_player: false,
            universe: String::new(),
            loaded_from_save: false,
            shop_catalog: dm.shop_catalog.clone(),
            last_action_header: String::new(),
            overworld: None,
        })
    }

    pub fn load_next_scenario(&mut self) -> Result<()> {
        self.game_manager.load_next_scenario()
    }

    /// Enter overworld mode: load `map_id` from `<root>/maps/`, place all
    /// active heroes at the map's default spawn, and switch `game_phase` to `Overworld`.
    ///
    /// If the player previously visited this map and the overworld state was
    /// preserved (i.e. `overworld` is already `Some` and its `map_id` matches),
    /// the saved positions are restored instead of resetting to spawn.
    pub fn enter_overworld(&mut self, map_id: &str, root: &Path) -> Result<()> {
        // Resume from preserved state if we already have it for this map.
        if let Some(ref mut ow) = self.overworld.as_mut().filter(|ow| ow.map_id == map_id) {
            // Mark any boss NPC whose fight scenario was just won as defeated.
            if self.game_manager.game_state.status == GameStatus::EndOfScenario {
                let won = self.game_manager.current_scenario.name.clone();
                if let Some(npc) = ow
                    .npcs
                    .iter_mut()
                    .find(|n| n.fight_scenario_id.as_deref() == Some(won.as_str()))
                {
                    npc.defeated = true;
                    ow.pending_fight = None;
                }
            }
            self.game_phase = GamePhase::Overworld;
            return Ok(());
        }
        self.enter_overworld_inner(map_id, None, root)
    }

    /// Like [`enter_overworld`] but place the hero at `spawn` instead of the
    /// map's default spawn point (used for door transitions).
    pub fn enter_overworld_at(
        &mut self,
        map_id: &str,
        spawn: crate::common::overworld::Position,
        root: &Path,
    ) -> Result<()> {
        self.enter_overworld_inner(map_id, Some(spawn), root)
    }

    fn enter_overworld_inner(
        &mut self,
        map_id: &str,
        spawn_override: Option<crate::common::overworld::Position>,
        root: &Path,
    ) -> Result<()> {
        let mut manager = OverworldManager::load_map(map_id, root)?;
        if let Some(spawn) = spawn_override {
            manager.spawn = spawn;
        }
        // Place every active hero at the (possibly overridden) spawn so that
        // all heroes have a valid position in `player_positions` and movement
        // lookups succeed regardless of which hero a player controls.
        for hero in &self.game_manager.pm.active_heroes {
            manager.place_hero_at_spawn(&hero.id_name);
        }
        self.overworld = Some(manager.state);
        self.game_phase = GamePhase::Overworld;
        Ok(())
    }

    /// Leave overworld and start a fight: look up `scenario_id` in `all_scenarios`,
    /// reset boss/game state, load bosses for the encounter, and switch to `Running`.
    pub fn exit_overworld_to_fight(&mut self, scenario_id: &str) {
        if let Some(scenario) = self
            .game_manager
            .all_scenarios
            .iter()
            .find(|s| s.name == scenario_id)
            .cloned()
        {
            self.game_manager.current_scenario = scenario;
        }
        // reset game/boss state so the encounter starts fresh
        self.game_manager.game_state.clear_scenario();
        self.game_manager.pm.clear_scenario();
        let all_bosses = self.game_manager.pm.all_bosses.clone();
        self.game_manager.set_active_bosses(&all_bosses);
        let _ = self.game_manager.start_new_turn();
        if let Some(ref mut ow) = self.overworld {
            ow.pending_encounter = None;
        }
        self.game_phase = GamePhase::Running;
    }
}

#[cfg(test)]
mod tests {
    use crate::common::constants::paths_const::{OFFLINE_ROOT, TEST_OFFLINE_ROOT};
    use crate::server::core_game_data::CoreGameData;
    use crate::server::data_manager::DataManager;
    use crate::server::scenario::Scenario;

    #[test]
    fn unit_new_with_universe_tagged_scenarios_succeeds() {
        // Regression: load_next_scenario must find the first scenario even when
        // all scenarios carry a non-empty universe (injected by DataManager from
        // the directory name). Before the fix it returned "No next scenario found"
        // because it compared current_universe "" against "lotr".
        let dm = DataManager::try_new(*TEST_OFFLINE_ROOT).unwrap();
        let lotr_scenario_1 = Scenario {
            name: "lotr_stage_1".to_string(),
            level: 1,
            universe: "lotr".to_string(),
            ..Scenario::default()
        };
        let lotr_scenario_2 = Scenario {
            name: "lotr_stage_2".to_string(),
            level: 2,
            universe: "lotr".to_string(),
            ..Scenario::default()
        };
        let result = CoreGameData::new_with_scenarios(
            &dm,
            "TestServer",
            vec![lotr_scenario_1, lotr_scenario_2],
        );
        assert!(
            result.is_ok(),
            "new_with_scenarios must succeed with universe-tagged scenarios: {:?}",
            result.err()
        );
    }

    #[test]
    fn unit_core_game_data_load_next_scenario() {
        let dm = DataManager::try_new(*TEST_OFFLINE_ROOT).unwrap();
        let mut core_game_data = CoreGameData::new(&dm, "Default").unwrap();
        let result = core_game_data.load_next_scenario();
        assert!(result.is_ok());
    }

    #[test]
    fn unit_load_next_scenario_resets_round_and_loads_bosses() {
        use crate::server::game_state::GameStatus;
        let dm = DataManager::try_new(*TEST_OFFLINE_ROOT).unwrap();
        let mut core = CoreGameData::new(&dm, "Default").unwrap();

        // Simulate the first scenario having run some rounds
        core.game_manager.game_state.current_round = 5;
        core.game_manager.game_state.status = GameStatus::EndOfScenario;

        core.load_next_scenario().unwrap();

        // Round counter must reset to 1: load_next_scenario starts the first round immediately
        assert_eq!(
            core.game_manager.game_state.current_round, 1,
            "round counter must be 1 at the start of a new scenario"
        );
        // At least one boss must be loaded for the new scenario
        assert!(
            !core.game_manager.pm.active_bosses.is_empty(),
            "new scenario must have at least one boss"
        );
        // The game must no longer be in EndOfScenario state
        assert_ne!(
            core.game_manager.game_state.status,
            GameStatus::EndOfScenario,
            "status must leave EndOfScenario after loading next scenario"
        );
    }

    #[test]
    fn unit_core_game_data_new() {
        let dm = DataManager::try_new(*TEST_OFFLINE_ROOT).unwrap();
        let core_game_data = CoreGameData::new(&dm, "Default");

        assert!(core_game_data.is_ok());
        let core_game_data = core_game_data.unwrap();
        assert_eq!(core_game_data.game_manager.pm.active_bosses.len(), 1);
        // check that the id_name of the boss is correctly set
        for boss in &core_game_data.game_manager.pm.active_bosses {
            assert!(boss.id_name.starts_with(&boss.db_full_name));
            assert!(boss.id_name.ends_with("_#1"));
        }
        assert_eq!(core_game_data.server_name, "Default");
        assert_eq!(
            core_game_data.game_phase,
            crate::server::server_manager::GamePhase::Default
        );
        assert_eq!(core_game_data.players_nb, 0);
        assert!(core_game_data.heroes_chosen.is_empty());
        assert!(core_game_data.game_manager.logs.is_empty());
        assert!(core_game_data.overworld.is_none());
    }

    #[test]
    fn unit_enter_overworld_at_uses_spawn_override() {
        use crate::common::overworld::Position;
        use crate::server::server_manager::GamePhase;

        let dm = DataManager::try_new(*TEST_OFFLINE_ROOT).unwrap();
        let mut core = CoreGameData::new(&dm, "Default").unwrap();

        let custom_spawn = Position::new(2, 3);
        let result = core.enter_overworld_at("pallet_town", custom_spawn.clone(), &OFFLINE_ROOT);
        assert!(
            result.is_ok(),
            "enter_overworld_at must succeed: {:?}",
            result.err()
        );
        assert_eq!(core.game_phase, GamePhase::Overworld);

        let ow = core.overworld.as_ref().unwrap();
        // The first hero must be placed at the custom spawn, not the map default.
        for pos in ow.player_positions.values() {
            assert_eq!(pos, &custom_spawn, "hero must be at the custom spawn");
        }
    }

    #[test]
    fn unit_enter_overworld_places_only_one_hero() {
        use crate::server::server_manager::GamePhase;

        let dm = DataManager::try_new(*TEST_OFFLINE_ROOT).unwrap();
        let mut core = CoreGameData::new(&dm, "Default").unwrap();
        let result = core.enter_overworld("pallet_town", &OFFLINE_ROOT);
        assert!(result.is_ok());
        assert_eq!(core.game_phase, GamePhase::Overworld);

        let ow = core.overworld.as_ref().unwrap();
        // All active heroes must appear in player_positions (one entry each).
        let hero_count = core.game_manager.pm.active_heroes.len();
        assert_eq!(
            ow.player_positions.len(),
            hero_count,
            "every active hero must have a position: expected {hero_count}, got {}",
            ow.player_positions.len()
        );
    }

    #[test]
    fn unit_enter_overworld_sets_phase_and_state() {
        use crate::server::server_manager::GamePhase;

        let dm = DataManager::try_new(*TEST_OFFLINE_ROOT).unwrap();
        let mut core = CoreGameData::new(&dm, "Default").unwrap();
        assert_eq!(core.game_phase, GamePhase::Default);

        let result = core.enter_overworld("pallet_town", &OFFLINE_ROOT);
        assert!(
            result.is_ok(),
            "enter_overworld must succeed: {:?}",
            result.err()
        );
        assert_eq!(core.game_phase, GamePhase::Overworld);
        let ow = core.overworld.as_ref().unwrap();
        assert_eq!(ow.map_id, "pallet_town");
        assert_eq!(ow.pending_encounter, None);
    }

    #[test]
    fn unit_enter_overworld_missing_map_returns_err() {
        let dm = DataManager::try_new(*TEST_OFFLINE_ROOT).unwrap();
        let mut core = CoreGameData::new(&dm, "Default").unwrap();
        assert!(
            core.enter_overworld("nonexistent_map", &OFFLINE_ROOT)
                .is_err()
        );
    }

    #[test]
    fn unit_exit_overworld_to_fight_sets_running() {
        use crate::server::server_manager::GamePhase;

        let dm = DataManager::try_new(*TEST_OFFLINE_ROOT).unwrap();
        let mut core = CoreGameData::new(&dm, "Default").unwrap();
        core.enter_overworld("pallet_town", &OFFLINE_ROOT).unwrap();
        assert_eq!(core.game_phase, GamePhase::Overworld);

        core.exit_overworld_to_fight("Stage 1");
        assert_eq!(core.game_phase, GamePhase::Running);
        assert_eq!(core.overworld.as_ref().unwrap().pending_encounter, None);
    }

    #[test]
    fn unit_game_phase_overworld_serde() {
        use crate::server::server_manager::GamePhase;

        let phase = GamePhase::Overworld;
        let json = serde_json::to_string(&phase).unwrap();
        let back: GamePhase = serde_json::from_str(&json).unwrap();
        assert_eq!(back, GamePhase::Overworld);
    }

    /// After winning the boss fight, re-entering the overworld marks that NPC as defeated.
    #[test]
    fn unit_enter_overworld_marks_boss_npc_defeated() {
        use crate::common::overworld::Position;
        use crate::server::game_state::GameStatus;
        use crate::server::overworld_manager::NpcState;
        use crate::server::server_manager::GamePhase;

        let dm = DataManager::try_new(*TEST_OFFLINE_ROOT).unwrap();
        let mut core = CoreGameData::new(&dm, "Default").unwrap();

        core.enter_overworld("pallet_town", &OFFLINE_ROOT).unwrap();

        // Inject a fake boss NPC into the overworld state.
        let ow = core.overworld.as_mut().unwrap();
        ow.npcs.push(NpcState {
            id: "boss_npc".to_string(),
            pos: Position::new(3, 3),
            dialog: vec!["Prepare yourself!".to_string()],
            fight_scenario_id: Some("lotr_stage_1".to_string()),
            defeated: false,
        });

        // Simulate a completed boss fight: set status and scenario name.
        core.game_manager.game_state.status = GameStatus::EndOfScenario;
        core.game_manager.current_scenario.name = "lotr_stage_1".to_string();
        core.game_phase = GamePhase::Running;

        // Re-enter the same map — should mark the boss NPC as defeated.
        core.enter_overworld("pallet_town", &OFFLINE_ROOT).unwrap();

        let ow = core.overworld.as_ref().unwrap();
        let boss = ow.npcs.iter().find(|n| n.id == "boss_npc").unwrap();
        assert!(
            boss.defeated,
            "boss NPC must be marked defeated after scenario win"
        );
        assert_eq!(ow.pending_fight, None, "pending_fight must be cleared");
    }

    /// After a fight triggered from the overworld, re-entering the same map
    /// must resume the saved state (not reload it from disk), preserving any
    /// in-memory mutations such as player positions or dialog state.
    #[test]
    fn unit_resume_overworld_after_fight_preserves_state() {
        use crate::server::server_manager::GamePhase;

        let dm = DataManager::try_new(*TEST_OFFLINE_ROOT).unwrap();
        let mut core = CoreGameData::new(&dm, "Default").unwrap();

        // Enter overworld.
        core.enter_overworld("pallet_town", &OFFLINE_ROOT).unwrap();
        assert_eq!(core.game_phase, GamePhase::Overworld);

        // Plant a marker in active_dialog so we can verify the resume path is taken
        // (a fresh load from disk would clear this field).
        core.overworld
            .as_mut()
            .unwrap()
            .active_dialog
            .push("marker".to_string());

        // Trigger a fight — game_phase → Running, overworld state must be kept.
        core.exit_overworld_to_fight("Stage 1");
        assert_eq!(core.game_phase, GamePhase::Running);
        assert!(
            core.overworld.is_some(),
            "overworld state must survive while fight is in progress"
        );

        // Re-enter the same map — enter_overworld detects the map_id match and resumes.
        core.enter_overworld("pallet_town", &OFFLINE_ROOT).unwrap();
        assert_eq!(core.game_phase, GamePhase::Overworld);

        assert!(
            core.overworld
                .as_ref()
                .unwrap()
                .active_dialog
                .contains(&"marker".to_string()),
            "active_dialog marker must survive fight + re-entry (state was resumed, not reloaded)"
        );
    }
}
