use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::common::overworld::{Direction, Position, TileKind};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcState {
    pub id: String,
    pub pos: Position,
    pub dialog: Vec<String>,
    /// If set, interacting with this NPC starts a fight instead of showing dialog.
    #[serde(default)]
    pub fight_scenario_id: Option<String>,
    /// True once the boss fight for this NPC has been won — the NPC is hidden from the map.
    #[serde(default)]
    pub defeated: bool,
}

/// Result returned by [`OverworldManager::interact`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InteractResult {
    /// Show the NPC dialog lines.
    Dialog(Vec<String>),
    /// Start a fight with the given scenario id.
    Fight(String),
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverworldState {
    pub map_id: String,
    /// key = hero id_name
    pub player_positions: HashMap<String, Position>,
    pub npcs: Vec<NpcState>,
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<Vec<TileKind>>,
    /// Set when a grass tile triggers an encounter; cleared when the fight begins.
    pub pending_encounter: Option<String>,
    /// Scenario ids that can be triggered by grass encounters on this map.
    pub encounters: Vec<String>,
    /// Dialog lines from the last NPC interaction; cleared on next move.
    #[serde(default)]
    pub active_dialog: Vec<String>,
    /// Set of "x_y" keys for door tiles that are currently locked.
    /// Stepping onto a locked door returns `Blocked` with a hint dialog.
    #[serde(default)]
    pub locked_doors: HashSet<String>,
    /// Scenario id queued to start after the boss's pre-fight dialog is dismissed.
    #[serde(default)]
    pub pending_fight: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MoveResult {
    Moved,
    Blocked,
    Encounter(String),
    MapTransition(String, Position),
}

#[derive(Debug, Clone, Deserialize)]
struct MapData {
    id: String,
    width: i32,
    height: i32,
    tiles: Vec<Vec<TileKind>>,
    npcs: Vec<NpcJson>,
    spawn: Position,
    encounters: Vec<String>,
    #[serde(default)]
    locked_doors: std::collections::HashSet<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct NpcJson {
    id: String,
    x: i32,
    y: i32,
    dialog: Vec<String>,
    #[serde(default)]
    fight_scenario_id: Option<String>,
}

/// Transient helper that wraps an [`OverworldState`] and provides movement logic.
/// Load with [`OverworldManager::load_map`]; persist only the inner `state`.
pub struct OverworldManager {
    pub state: OverworldState,
    pub spawn: Position,
}

impl OverworldManager {
    /// Load a map from `<root>/maps/<map_id>.json` and return a ready manager.
    pub fn load_map(map_id: &str, root: &Path) -> Result<OverworldManager> {
        let map_path = root.join("maps").join(format!("{map_id}.json"));
        let content = std::fs::read_to_string(&map_path)?;
        let map: MapData = serde_json::from_str(&content)?;

        let npcs = map
            .npcs
            .iter()
            .map(|n| NpcState {
                id: n.id.clone(),
                pos: Position::new(n.x, n.y),
                dialog: n.dialog.clone(),
                fight_scenario_id: n.fight_scenario_id.clone(),
                defeated: false,
            })
            .collect();

        let state = OverworldState {
            map_id: map.id,
            player_positions: HashMap::new(),
            npcs,
            width: map.width,
            height: map.height,
            tiles: map.tiles,
            pending_encounter: None,
            encounters: map.encounters,
            active_dialog: Vec::new(),
            locked_doors: map.locked_doors,
            pending_fight: None,
        };

        Ok(OverworldManager {
            state,
            spawn: map.spawn,
        })
    }

    /// Reconstruct a manager from a persisted state (spawn defaults to origin).
    pub fn from_state(state: OverworldState) -> Self {
        OverworldManager {
            state,
            spawn: Position::default(),
        }
    }

    /// Return `true` if `pos` is within bounds and on a passable tile (not Wall or Water).
    pub fn is_passable(&self, pos: &Position) -> bool {
        if pos.x < 0 || pos.y < 0 || pos.x >= self.state.width || pos.y >= self.state.height {
            return false;
        }
        !matches!(
            self.state
                .tiles
                .get(pos.y as usize)
                .and_then(|r| r.get(pos.x as usize)),
            Some(TileKind::Wall) | Some(TileKind::Water) | None
        )
    }

    /// Place `hero_id` at the map spawn point.
    /// If the spawn tile is a wall or out-of-bounds, falls back to the first
    /// passable tile found in row-major order.
    pub fn place_hero_at_spawn(&mut self, hero_id: &str) {
        let spawn = if self.is_passable(&self.spawn) {
            self.spawn.clone()
        } else {
            tracing::warn!(
                "Spawn {:?} is not passable on map '{}'; using first passable tile",
                self.spawn,
                self.state.map_id
            );
            (0..self.state.height)
                .flat_map(|y| (0..self.state.width).map(move |x| Position::new(x, y)))
                .find(|p| self.is_passable(p))
                .unwrap_or_else(|| self.spawn.clone())
        };
        self.state
            .player_positions
            .insert(hero_id.to_string(), spawn);
    }

    /// Move `hero_id` one step in `dir`.
    ///
    /// Returns:
    /// - `Blocked` — wall, water, out-of-bounds, or unknown hero
    /// - `Moved` — free tile or grass with no encounter roll
    /// - `Encounter(scenario_id)` — grass tile triggered a fight (50 % chance)
    /// - `MapTransition(map_id, spawn)` — hero stepped on a door
    pub fn move_player(&mut self, hero_id: &str, dir: Direction) -> MoveResult {
        let Some(current_pos) = self.state.player_positions.get(hero_id).cloned() else {
            return MoveResult::Blocked;
        };

        let new_pos = match dir {
            Direction::Up => Position::new(current_pos.x, current_pos.y - 1),
            Direction::Down => Position::new(current_pos.x, current_pos.y + 1),
            Direction::Left => Position::new(current_pos.x - 1, current_pos.y),
            Direction::Right => Position::new(current_pos.x + 1, current_pos.y),
        };

        if new_pos.x < 0
            || new_pos.y < 0
            || new_pos.x >= self.state.width
            || new_pos.y >= self.state.height
        {
            return MoveResult::Blocked;
        }

        // Block movement onto a tile occupied by another player or an NPC.
        if self.is_occupied_by_other(&new_pos, hero_id) {
            return MoveResult::Blocked;
        }

        let tile_kind = match self
            .state
            .tiles
            .get(new_pos.y as usize)
            .and_then(|row| row.get(new_pos.x as usize))
        {
            Some(k) => k.clone(),
            None => return MoveResult::Blocked,
        };

        match tile_kind {
            TileKind::Wall | TileKind::Water => MoveResult::Blocked,
            TileKind::Floor => {
                self.state
                    .player_positions
                    .insert(hero_id.to_string(), new_pos);
                MoveResult::Moved
            }
            TileKind::Grass => {
                self.state
                    .player_positions
                    .insert(hero_id.to_string(), new_pos);
                if !self.state.encounters.is_empty() {
                    let mut rng = rand::rng();
                    if rng.random_bool(0.5) {
                        let idx = rng.random_range(0..self.state.encounters.len());
                        let scenario_id = self.state.encounters[idx].clone();
                        self.state.pending_encounter = Some(scenario_id.clone());
                        return MoveResult::Encounter(scenario_id);
                    }
                }
                MoveResult::Moved
            }
            TileKind::Door { target_map, spawn } => {
                let door_key = format!("{}_{}", new_pos.x, new_pos.y);
                if self.state.locked_doors.contains(&door_key) {
                    self.state.active_dialog = vec![
                        "⛔ The passage is sealed.".to_string(),
                        "Defeat the enemies first!".to_string(),
                    ];
                    return MoveResult::Blocked;
                }
                MoveResult::MapTransition(target_map, spawn)
            }
        }
    }

    /// Return `true` if `pos` is occupied by any player or living NPC on this map.
    pub fn is_occupied(&self, pos: &Position) -> bool {
        self.state.player_positions.values().any(|p| p == pos)
            || self
                .state
                .npcs
                .iter()
                .any(|npc| !npc.defeated && &npc.pos == pos)
    }

    /// Return `true` if `pos` is occupied by a player other than `hero_id`, or by any living NPC.
    fn is_occupied_by_other(&self, pos: &Position, hero_id: &str) -> bool {
        self.state
            .player_positions
            .iter()
            .any(|(id, p)| id.as_str() != hero_id && p == pos)
            || self
                .state
                .npcs
                .iter()
                .any(|npc| !npc.defeated && &npc.pos == pos)
    }

    /// Interact with the first living NPC adjacent (4-directional) to `hero_id`.
    ///
    /// For boss NPCs with dialog: the first call shows dialog and queues the fight via
    /// `pending_fight`; the second call (or any subsequent interact) triggers the fight.
    /// For boss NPCs without dialog: starts the fight immediately.
    /// For friendly NPCs: shows their dialog lines.
    /// Returns `None` when no adjacent living NPC is found.
    pub fn interact(&mut self, hero_id: &str) -> Option<InteractResult> {
        // A pending fight (queued after dialog) takes priority.
        if let Some(scenario_id) = self.state.pending_fight.take() {
            self.state.active_dialog.clear();
            return Some(InteractResult::Fight(scenario_id));
        }

        let pos = self.state.player_positions.get(hero_id)?;
        let adjacent = [
            Position::new(pos.x, pos.y - 1),
            Position::new(pos.x, pos.y + 1),
            Position::new(pos.x - 1, pos.y),
            Position::new(pos.x + 1, pos.y),
        ];
        let npc = self
            .state
            .npcs
            .iter()
            .find(|npc| !npc.defeated && adjacent.contains(&npc.pos))?;

        if let Some(ref scenario_id) = npc.fight_scenario_id {
            if npc.dialog.is_empty() {
                Some(InteractResult::Fight(scenario_id.clone()))
            } else {
                let lines = npc.dialog.clone();
                let sid = scenario_id.clone();
                self.state.active_dialog = lines.clone();
                self.state.pending_fight = Some(sid);
                Some(InteractResult::Dialog(lines))
            }
        } else {
            let dialog = npc.dialog.clone();
            self.state.active_dialog = dialog.clone();
            Some(InteractResult::Dialog(dialog))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_map(content: &str, map_id: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rpg_map_test_{}_{}",
            map_id,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        std::fs::create_dir_all(dir.join("maps")).unwrap();
        let path = dir.join("maps").join(format!("{map_id}.json"));
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        dir
    }

    fn small_map_json() -> &'static str {
        r#"{
  "id": "test_map",
  "width": 5,
  "height": 5,
  "tiles": [
    ["wall","wall","wall","wall","wall"],
    ["wall","floor","floor","floor","wall"],
    ["wall","floor","grass","floor","wall"],
    ["wall","water","floor",{"door":{"target_map":"route_1","spawn":{"x":1,"y":1}}},"wall"],
    ["wall","wall","wall","wall","wall"]
  ],
  "npcs": [{"id":"elder","x":1,"y":1,"dialog":["Hello!","Be careful."]}],
  "spawn": {"x":2,"y":1},
  "encounters": ["stage_1"]
}"#
    }

    #[test]
    fn unit_load_map_ok() {
        let root = write_temp_map(small_map_json(), "test_map");
        let mgr = OverworldManager::load_map("test_map", &root).unwrap();
        assert_eq!(mgr.state.map_id, "test_map");
        assert_eq!(mgr.state.width, 5);
        assert_eq!(mgr.state.height, 5);
        assert_eq!(mgr.spawn, Position::new(2, 1));
        assert_eq!(mgr.state.npcs.len(), 1);
        assert_eq!(mgr.state.encounters, vec!["stage_1"]);
    }

    #[test]
    fn unit_load_map_missing_file() {
        let root = std::path::PathBuf::from("/nonexistent/path");
        assert!(OverworldManager::load_map("no_map", &root).is_err());
    }

    #[test]
    fn unit_move_player_floor() {
        let root = write_temp_map(small_map_json(), "test_map_floor");
        let mut mgr = OverworldManager::load_map("test_map_floor", &root).unwrap();
        mgr.place_hero_at_spawn("hero_1");

        // spawn is (2,1) — move right to (3,1) which is floor
        let result = mgr.move_player("hero_1", Direction::Right);
        assert_eq!(result, MoveResult::Moved);
        assert_eq!(
            *mgr.state.player_positions.get("hero_1").unwrap(),
            Position::new(3, 1)
        );
    }

    #[test]
    fn unit_move_player_blocked_wall() {
        let root = write_temp_map(small_map_json(), "test_map_wall");
        let mut mgr = OverworldManager::load_map("test_map_wall", &root).unwrap();
        mgr.place_hero_at_spawn("hero_1");

        // spawn (2,1) — move up to (2,0) which is wall
        let result = mgr.move_player("hero_1", Direction::Up);
        assert_eq!(result, MoveResult::Blocked);
        assert_eq!(
            *mgr.state.player_positions.get("hero_1").unwrap(),
            Position::new(2, 1)
        );
    }

    #[test]
    fn unit_move_player_blocked_water() {
        let root = write_temp_map(small_map_json(), "test_map_water");
        let mut mgr = OverworldManager::load_map("test_map_water", &root).unwrap();
        // place hero at (2,2) — grass tile — then move left to (1,3) water
        mgr.state
            .player_positions
            .insert("hero_1".to_string(), Position::new(2, 3));
        let result = mgr.move_player("hero_1", Direction::Left);
        assert_eq!(result, MoveResult::Blocked);
    }

    #[test]
    fn unit_move_player_blocked_bounds() {
        let root = write_temp_map(small_map_json(), "test_map_bounds");
        let mut mgr = OverworldManager::load_map("test_map_bounds", &root).unwrap();
        // place hero at left wall (0,2)
        mgr.state
            .player_positions
            .insert("hero_1".to_string(), Position::new(0, 2));
        let result = mgr.move_player("hero_1", Direction::Left);
        assert_eq!(result, MoveResult::Blocked);
    }

    #[test]
    fn unit_move_player_grass_returns_moved_or_encounter() {
        let root = write_temp_map(small_map_json(), "test_map_grass");
        let mut mgr = OverworldManager::load_map("test_map_grass", &root).unwrap();
        // place hero at (2,1); move down to (2,2) = grass
        mgr.place_hero_at_spawn("hero_1");
        let result = mgr.move_player("hero_1", Direction::Down);
        assert!(
            matches!(result, MoveResult::Moved | MoveResult::Encounter(_)),
            "grass must give Moved or Encounter, got {result:?}"
        );
        // hero must have moved to (2,2) regardless
        assert_eq!(
            *mgr.state.player_positions.get("hero_1").unwrap(),
            Position::new(2, 2)
        );
    }

    #[test]
    fn unit_move_player_grass_no_encounters() {
        let json = r#"{
  "id":"t","width":3,"height":3,
  "tiles":[["wall","wall","wall"],
            ["wall","floor","wall"],
            ["wall","grass","wall"]],
  "npcs":[],"spawn":{"x":1,"y":1},"encounters":[]
}"#;
        let root = write_temp_map(json, "t");
        let mut mgr = OverworldManager::load_map("t", &root).unwrap();
        mgr.place_hero_at_spawn("h");
        // With empty encounters list, grass always returns Moved
        let result = mgr.move_player("h", Direction::Down);
        assert_eq!(result, MoveResult::Moved);
    }

    #[test]
    fn unit_move_player_door() {
        let root = write_temp_map(small_map_json(), "test_map_door");
        let mut mgr = OverworldManager::load_map("test_map_door", &root).unwrap();
        // place hero at (2,3) to step right to (3,3) = door
        mgr.state
            .player_positions
            .insert("hero_1".to_string(), Position::new(2, 3));
        let result = mgr.move_player("hero_1", Direction::Right);
        assert!(
            matches!(result, MoveResult::MapTransition(ref map, _) if map == "route_1"),
            "expected MapTransition to route_1, got {result:?}"
        );
    }

    #[test]
    fn unit_move_player_unknown_hero() {
        let root = write_temp_map(small_map_json(), "test_map_unk");
        let mut mgr = OverworldManager::load_map("test_map_unk", &root).unwrap();
        let result = mgr.move_player("ghost", Direction::Up);
        assert_eq!(result, MoveResult::Blocked);
    }

    #[test]
    fn unit_interact_with_adjacent_npc() {
        let root = write_temp_map(small_map_json(), "test_map_npc");
        let mut mgr = OverworldManager::load_map("test_map_npc", &root).unwrap();
        // NPC is at (1,1), place hero at (2,1) — right of NPC
        mgr.place_hero_at_spawn("hero_1");
        let result = mgr.interact("hero_1");
        assert_eq!(
            result,
            Some(InteractResult::Dialog(vec![
                "Hello!".to_string(),
                "Be careful.".to_string()
            ]))
        );
    }

    #[test]
    fn unit_interact_no_adjacent_npc() {
        let root = write_temp_map(small_map_json(), "test_map_nonpc");
        let mut mgr = OverworldManager::load_map("test_map_nonpc", &root).unwrap();
        // NPC is at (1,1), place hero far away at (3,1)
        mgr.state
            .player_positions
            .insert("hero_1".to_string(), Position::new(3, 1));
        let result = mgr.interact("hero_1");
        assert!(result.is_none());
    }

    #[test]
    fn unit_interact_unknown_hero() {
        let root = write_temp_map(small_map_json(), "test_map_unk2");
        let mut mgr = OverworldManager::load_map("test_map_unk2", &root).unwrap();
        assert!(mgr.interact("ghost").is_none());
    }

    #[test]
    fn unit_interact_enemy_npc_triggers_fight() {
        let enemy_map = r#"{
  "id": "enemy_map",
  "width": 5,
  "height": 5,
  "tiles": [
    ["wall","wall","wall","wall","wall"],
    ["wall","floor","floor","floor","wall"],
    ["wall","floor","floor","floor","wall"],
    ["wall","floor","floor","floor","wall"],
    ["wall","wall","wall","wall","wall"]
  ],
  "npcs": [{"id":"goblin","x":1,"y":1,"dialog":[],"fight_scenario_id":"Patrouille Gobeline"}],
  "spawn": {"x":2,"y":1},
  "encounters": []
}"#;
        let root = write_temp_map(enemy_map, "enemy_map");
        let mut mgr = OverworldManager::load_map("enemy_map", &root).unwrap();
        mgr.place_hero_at_spawn("hero_1");
        // hero is at (2,1), goblin is at (1,1) — adjacent left
        let result = mgr.interact("hero_1");
        assert_eq!(
            result,
            Some(InteractResult::Fight("Patrouille Gobeline".to_string()))
        );
    }

    #[test]
    fn unit_from_state_roundtrip() {
        let root = write_temp_map(small_map_json(), "test_map_rt");
        let mut mgr = OverworldManager::load_map("test_map_rt", &root).unwrap();
        mgr.place_hero_at_spawn("h");
        let state = mgr.state.clone();
        let mgr2 = OverworldManager::from_state(state.clone());
        assert_eq!(mgr2.state, state);
    }

    // ── is_occupied tests ────────────────────────────────────────────────────

    #[test]
    fn unit_is_occupied_free_tile() {
        let root = write_temp_map(small_map_json(), "test_map_occ_free");
        let mgr = OverworldManager::load_map("test_map_occ_free", &root).unwrap();
        // No heroes placed — every tile is free.
        assert!(!mgr.is_occupied(&Position::new(2, 1)));
    }

    #[test]
    fn unit_is_occupied_by_player() {
        let root = write_temp_map(small_map_json(), "test_map_occ_player");
        let mut mgr = OverworldManager::load_map("test_map_occ_player", &root).unwrap();
        mgr.place_hero_at_spawn("hero_1"); // spawn = (2,1)
        assert!(mgr.is_occupied(&Position::new(2, 1)));
        // Adjacent tile is free.
        assert!(!mgr.is_occupied(&Position::new(3, 1)));
    }

    #[test]
    fn unit_is_occupied_by_npc() {
        // NPC "elder" is at (1,1) in small_map_json.
        let root = write_temp_map(small_map_json(), "test_map_occ_npc");
        let mgr = OverworldManager::load_map("test_map_occ_npc", &root).unwrap();
        assert!(mgr.is_occupied(&Position::new(1, 1)));
        assert!(!mgr.is_occupied(&Position::new(2, 1)));
    }

    #[test]
    fn unit_move_blocked_by_other_player() {
        let root = write_temp_map(small_map_json(), "test_map_occ_block");
        let mut mgr = OverworldManager::load_map("test_map_occ_block", &root).unwrap();
        // Place hero_1 at (2,1) and hero_2 at (3,1).
        mgr.state
            .player_positions
            .insert("hero_1".to_string(), Position::new(2, 1));
        mgr.state
            .player_positions
            .insert("hero_2".to_string(), Position::new(3, 1));
        // hero_1 tries to move right onto hero_2 — must be blocked.
        assert_eq!(
            mgr.move_player("hero_1", Direction::Right),
            MoveResult::Blocked
        );
        // Position must be unchanged.
        assert_eq!(
            *mgr.state.player_positions.get("hero_1").unwrap(),
            Position::new(2, 1)
        );
    }

    #[test]
    fn unit_move_blocked_by_npc() {
        // NPC "elder" is at (1,1) in small_map_json. Hero at (2,1) tries to move left.
        let root = write_temp_map(small_map_json(), "test_map_occ_npc_block");
        let mut mgr = OverworldManager::load_map("test_map_occ_npc_block", &root).unwrap();
        mgr.place_hero_at_spawn("hero_1"); // spawn = (2,1)
        assert_eq!(
            mgr.move_player("hero_1", Direction::Left),
            MoveResult::Blocked
        );
        assert_eq!(
            *mgr.state.player_positions.get("hero_1").unwrap(),
            Position::new(2, 1)
        );
    }

    // ── defeated NPC / boss dialog tests ────────────────────────────────────

    fn boss_map_state(boss_dialog: Vec<String>) -> OverworldState {
        OverworldState {
            map_id: "boss_map".to_string(),
            player_positions: [("hero".to_string(), Position::new(2, 1))]
                .into_iter()
                .collect(),
            npcs: vec![NpcState {
                id: "boss".to_string(),
                pos: Position::new(1, 1),
                dialog: boss_dialog,
                fight_scenario_id: Some("boss_fight".to_string()),
                defeated: false,
            }],
            width: 5,
            height: 5,
            tiles: vec![
                vec![
                    TileKind::Wall,
                    TileKind::Wall,
                    TileKind::Wall,
                    TileKind::Wall,
                    TileKind::Wall,
                ],
                vec![
                    TileKind::Wall,
                    TileKind::Floor,
                    TileKind::Floor,
                    TileKind::Floor,
                    TileKind::Wall,
                ],
                vec![
                    TileKind::Wall,
                    TileKind::Wall,
                    TileKind::Wall,
                    TileKind::Wall,
                    TileKind::Wall,
                ],
                vec![
                    TileKind::Wall,
                    TileKind::Wall,
                    TileKind::Wall,
                    TileKind::Wall,
                    TileKind::Wall,
                ],
                vec![
                    TileKind::Wall,
                    TileKind::Wall,
                    TileKind::Wall,
                    TileKind::Wall,
                    TileKind::Wall,
                ],
            ],
            pending_encounter: None,
            encounters: vec![],
            active_dialog: vec![],
            locked_doors: Default::default(),
            pending_fight: None,
        }
    }

    #[test]
    fn unit_interact_boss_with_dialog_shows_dialog_then_fight() {
        let state = boss_map_state(vec!["I will destroy you!".to_string()]);
        let mut mgr = OverworldManager::from_state(state);

        // First interact: shows dialog, queues fight.
        let result = mgr.interact("hero");
        assert_eq!(
            result,
            Some(InteractResult::Dialog(vec![
                "I will destroy you!".to_string()
            ]))
        );
        assert_eq!(mgr.state.pending_fight, Some("boss_fight".to_string()));

        // Second interact: triggers the queued fight.
        let result = mgr.interact("hero");
        assert_eq!(
            result,
            Some(InteractResult::Fight("boss_fight".to_string()))
        );
        assert_eq!(mgr.state.pending_fight, None);
        assert!(mgr.state.active_dialog.is_empty());
    }

    #[test]
    fn unit_interact_boss_without_dialog_fights_immediately() {
        let state = boss_map_state(vec![]);
        let mut mgr = OverworldManager::from_state(state);

        let result = mgr.interact("hero");
        assert_eq!(
            result,
            Some(InteractResult::Fight("boss_fight".to_string()))
        );
        assert_eq!(mgr.state.pending_fight, None);
    }

    #[test]
    fn unit_defeated_npc_not_occupied() {
        let root = write_temp_map(small_map_json(), "test_map_defeated_occ");
        let mut mgr = OverworldManager::load_map("test_map_defeated_occ", &root).unwrap();

        // NPC "elder" is at (1,1) — initially occupied.
        assert!(mgr.is_occupied(&Position::new(1, 1)));

        mgr.state.npcs.iter_mut().for_each(|n| n.defeated = true);

        assert!(!mgr.is_occupied(&Position::new(1, 1)));
    }

    #[test]
    fn unit_hero_can_move_through_defeated_npc_tile() {
        let root = write_temp_map(small_map_json(), "test_map_defeated_mv");
        let mut mgr = OverworldManager::load_map("test_map_defeated_mv", &root).unwrap();
        mgr.place_hero_at_spawn("hero_1"); // spawn = (2,1)

        // Initially blocked by living NPC at (1,1).
        assert_eq!(
            mgr.move_player("hero_1", Direction::Left),
            MoveResult::Blocked
        );

        // Mark NPC defeated — hero should now be able to step onto (1,1).
        mgr.state.npcs.iter_mut().for_each(|n| n.defeated = true);
        assert_eq!(
            mgr.move_player("hero_1", Direction::Left),
            MoveResult::Moved
        );
        assert_eq!(
            *mgr.state.player_positions.get("hero_1").unwrap(),
            Position::new(1, 1)
        );
    }

    // ── lotr_shire map layout tests ──────────────────────────────────────────
    //
    // Map: 14×10. Outer border = wall (x=0,x=13,y=0,y=9).
    // Inner-left border = wall at x=1 for all interior rows.
    // Diagonal interior walls: (4,1),(5,2),(6,3),(7,4),(8,5),(9,6).
    // Spawn: (2,7). Gandalf NPC: (2,2). Goblin NPC: (10,3).
    //
    //   (1,1) → wall  (inner-left border)
    //   (4,2) → floor (left of diagonal gap)
    fn lotr_shire_test_json() -> &'static str {
        r#"{
  "id": "lotr_shire_test",
  "width": 14,
  "height": 10,
  "tiles": [
    ["wall","wall","wall","wall","wall","wall","wall",{"door":{"target_map":"lotr_misty_mountains","spawn":{"x":7,"y":8}}},"wall","wall","wall","wall","wall","wall"],
    ["wall","wall","floor","floor","wall","floor","floor","floor","floor","floor","floor","floor","floor","wall"],
    ["wall","wall","floor","floor","floor","wall","floor","floor","floor","floor","floor","floor","floor","wall"],
    ["wall","wall","floor","floor","floor","floor","wall","floor","floor","floor","floor","floor","floor","wall"],
    ["wall","wall","floor","floor","floor","floor","floor","wall","floor","floor","floor","floor","floor","wall"],
    ["wall","wall","floor","floor","floor","floor","floor","floor","wall","floor","floor","floor","floor","wall"],
    ["wall","wall","floor","floor","floor","floor","floor","floor","floor","wall","floor","floor","floor","wall"],
    ["wall","wall","floor","floor","floor","floor","floor","floor","floor","floor","floor","floor","floor","wall"],
    ["wall","wall","floor","floor","floor","floor","floor","floor","floor","floor","floor","floor","floor","wall"],
    ["wall","wall","wall","wall","wall","wall","wall","wall","wall","wall","wall","wall","wall","wall"]
  ],
  "npcs": [
    {"id":"gandalf","x":2,"y":2,"dialog":["La Comté est menacée, ami !"]},
    {"id":"gobelin_eclaireur","x":10,"y":3,"dialog":["Combat!"],"fight_scenario_id":"Patrouille Gobeline"}
  ],
  "spawn": {"x":2,"y":7},
  "encounters": []
}"#
    }

    /// (1,1) is the inner-left-border wall; moving left into it must be Blocked.
    #[test]
    fn unit_lotr_shire_wall_at_1_1_blocks() {
        let root = write_temp_map(lotr_shire_test_json(), "lotr_shire_test_wall");
        let mut mgr = OverworldManager::load_map("lotr_shire_test_wall", &root).unwrap();
        // Place hero one step right of (1,1)
        mgr.state
            .player_positions
            .insert("hero".to_string(), Position::new(2, 1));
        assert_eq!(
            mgr.move_player("hero", Direction::Left),
            MoveResult::Blocked,
            "(1,1) should be a wall – movement must be blocked"
        );
        // Position must be unchanged
        assert_eq!(
            *mgr.state.player_positions.get("hero").unwrap(),
            Position::new(2, 1)
        );
    }

    /// (4,2) is a floor tile; moving right onto it must succeed.
    #[test]
    fn unit_lotr_shire_floor_at_4_2_passable() {
        let root = write_temp_map(lotr_shire_test_json(), "lotr_shire_test_floor");
        let mut mgr = OverworldManager::load_map("lotr_shire_test_floor", &root).unwrap();
        // Place hero one step left of (4,2)
        mgr.state
            .player_positions
            .insert("hero".to_string(), Position::new(3, 2));
        assert_eq!(
            mgr.move_player("hero", Direction::Right),
            MoveResult::Moved,
            "(4,2) should be floor – movement must succeed"
        );
        assert_eq!(
            *mgr.state.player_positions.get("hero").unwrap(),
            Position::new(4, 2)
        );
    }

    /// The map spawn point must be on a passable (floor) tile.
    #[test]
    fn unit_lotr_shire_spawn_on_floor() {
        let root = write_temp_map(lotr_shire_test_json(), "lotr_shire_test_spawn");
        let mgr = OverworldManager::load_map("lotr_shire_test_spawn", &root).unwrap();
        assert!(
            mgr.is_passable(&mgr.spawn),
            "Spawn {:?} must be on a floor tile, not a wall",
            mgr.spawn
        );
    }

    /// Every NPC must be placed on a floor tile (not a wall).
    #[test]
    fn unit_lotr_shire_npcs_on_floor() {
        let root = write_temp_map(lotr_shire_test_json(), "lotr_shire_test_npcs");
        let mgr = OverworldManager::load_map("lotr_shire_test_npcs", &root).unwrap();
        for npc in &mgr.state.npcs {
            assert!(
                mgr.is_passable(&npc.pos),
                "NPC '{}' at {:?} must be on a floor tile, not a wall",
                npc.id,
                npc.pos
            );
        }
    }

    /// Every diagonal wall tile must block movement from the adjacent floor tile.
    /// Diagonal: (4,1),(5,2),(6,3),(7,4),(8,5),(9,6).
    #[test]
    fn unit_lotr_shire_diagonal_wall_blocked() {
        let root = write_temp_map(lotr_shire_test_json(), "lotr_shire_test_diag");
        let mut mgr = OverworldManager::load_map("lotr_shire_test_diag", &root).unwrap();

        // Each entry: (hero_start_x, hero_start_y, direction, wall_label)
        let cases: &[(i32, i32, Direction, &str)] = &[
            (3, 1, Direction::Right, "(4,1)"), // → hits diagonal wall at x=4,y=1
            (4, 2, Direction::Right, "(5,2)"), // → hits diagonal wall at x=5,y=2
            (5, 3, Direction::Right, "(6,3)"), // → hits diagonal wall at x=6,y=3
            (6, 4, Direction::Right, "(7,4)"), // → hits diagonal wall at x=7,y=4
            (7, 5, Direction::Right, "(8,5)"), // → hits diagonal wall at x=8,y=5
            (8, 6, Direction::Right, "(9,6)"), // → hits diagonal wall at x=9,y=6
        ];
        for &(sx, sy, ref dir, label) in cases {
            mgr.state
                .player_positions
                .insert("hero".to_string(), Position::new(sx, sy));
            assert_eq!(
                mgr.move_player("hero", dir.clone()),
                MoveResult::Blocked,
                "Diagonal wall {} should block movement",
                label
            );
        }
    }
}
