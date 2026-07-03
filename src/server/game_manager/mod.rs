use std::{collections::HashMap, path::Path};

use crate::{
    character_mod::equipment::{Equipment, EquipmentJsonKey},
    common::{constants::paths_const::*, lang::Lang, log_data::LogData},
    server::{
        end_of_scenario::EndOfScenario,
        game_paths::GamePaths,
        game_state::GameState,
        players_manager::{DodgeInfo, GameAtkEffect, PlayerManager},
        scenario::{Scenario, ScenarioState},
    },
};
use serde::{Deserialize, Serialize};

mod attack_flow;
mod scenario_flow;
mod turn_flow;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultLaunchAttack {
    pub launcher_id_name: String,
    pub atk_name: String,
    pub new_game_atk_effects: Vec<GameAtkEffect>,
    pub is_crit: bool,
    pub all_dodging: Vec<DodgeInfo>,
    pub is_boss_atk: bool,
    pub logs_end_of_round: Vec<LogData>,
    pub logs_atk: Vec<LogData>,
    /// Passive-triggered log entries for this turn (e.g. IsDamageTxHealNeedyAlly heal).
    /// Included in `logs_atk` for the log sheet and also exposed here for the gameboard.
    pub passive_logs: Vec<LogData>,
    pub turn_nb: usize,
    pub round_nb: usize,
    /// True when the finishing blow was delivered by a damage-over-time effect (regen tick), not the direct attack.
    pub is_dot_kill: bool,
    /// Last attack name of the character killed by DOT (empty if not a DOT kill).
    pub dying_char_last_atk: String,
}

impl ResultLaunchAttack {
    /// Locale-specific attack name; falls back to the canonical `atk_name` when
    /// there's no landed effect to resolve an `AttackType` from (e.g. an attack
    /// whose only effects were fully dodged/blocked).
    pub fn atk_display_name_for(&self, lang: Lang) -> &str {
        self.new_game_atk_effects
            .first()
            .map(|gae| gae.atk_type.name_for(lang))
            .unwrap_or(&self.atk_name)
    }
}

/// The entry of the library.
/// That object should be called to access to all the different functionalities.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameManager {
    /// Game state of the current game
    pub game_state: GameState,
    /// Player manager
    pub pm: PlayerManager,
    /// Paths of the current game
    pub game_paths: GamePaths,
    /// logs of the game, to display in the log sheet
    pub logs: Vec<LogData>,
    /// Current scenario of the game, to adapt the behavior of the fight
    pub current_scenario: Scenario,
    /// all scenarios
    pub all_scenarios: Vec<Scenario>,
    /// State of the different scenarios, to know which scenario is available for the player and to adapt the behavior of the fight
    pub states_scenarios: HashMap<String, ScenarioState>,
    /// End of scenario
    pub end_of_scenario: EndOfScenario,
}

impl GameManager {
    /// Create a new game manager with the given path for the offline files and the default active characters
    pub fn new<P: AsRef<Path>>(
        path: P,
        equipment_table: HashMap<EquipmentJsonKey, Vec<Equipment>>,
        scenarios: Vec<Scenario>,
    ) -> GameManager {
        // if path is empty, use the default one
        let mut new_path = path.as_ref();
        if new_path.as_os_str().is_empty() {
            new_path = &OFFLINE_ROOT;
        }
        // create game state
        let game_state = GameState::new();
        let game_name = game_state.game_name.clone();

        // scenarios state
        let mut states_scenarios = HashMap::new();
        for scenario in &scenarios {
            states_scenarios.insert(scenario.name.clone(), ScenarioState::NotStarted);
        }

        GameManager {
            game_state,
            pm: PlayerManager::new(equipment_table),
            game_paths: GamePaths::new(new_path, &game_name),
            logs: Vec::new(),
            current_scenario: Scenario::default(),
            all_scenarios: scenarios,
            states_scenarios,
            end_of_scenario: EndOfScenario::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character_mod::attack_type::AttackType;

    #[test]
    fn unit_atk_display_name_for_fallback() {
        let result = ResultLaunchAttack {
            atk_name: "Charge".to_owned(),
            new_game_atk_effects: vec![GameAtkEffect {
                atk_type: AttackType {
                    name: "Charge".to_owned(),
                    ..Default::default()
                },
                ..Default::default()
            }],
            ..Default::default()
        };
        assert_eq!(result.atk_display_name_for(Lang::En), "Charge");
        assert_eq!(result.atk_display_name_for(Lang::Fr), "Charge");
    }

    #[test]
    fn unit_atk_display_name_for_localized() {
        let result = ResultLaunchAttack {
            atk_name: "Don de vie".to_owned(),
            new_game_atk_effects: vec![GameAtkEffect {
                atk_type: AttackType {
                    name: "Don de vie".to_owned(),
                    name_en: "Gift of Life".to_owned(),
                    name_fr: "Don de vie".to_owned(),
                    ..Default::default()
                },
                ..Default::default()
            }],
            ..Default::default()
        };
        assert_eq!(result.atk_display_name_for(Lang::En), "Gift of Life");
        assert_eq!(result.atk_display_name_for(Lang::Fr), "Don de vie");
    }

    #[test]
    fn unit_atk_display_name_for_no_effects_falls_back_to_atk_name() {
        let result = ResultLaunchAttack {
            atk_name: "Charge".to_owned(),
            new_game_atk_effects: vec![],
            ..Default::default()
        };
        assert_eq!(result.atk_display_name_for(Lang::En), "Charge");
        assert_eq!(result.atk_display_name_for(Lang::Fr), "Charge");
    }
}
