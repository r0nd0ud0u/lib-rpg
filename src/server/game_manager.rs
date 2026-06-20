use std::{collections::HashMap, path::Path};

use crate::{
    character_mod::{
        attack_type::{AttackType, LauncherAtkInfo},
        buffers::BufKinds,
        character::{Character, CharacterKind},
        class::Class,
        effect::{build_energy_effect, build_hp_effect, build_resurrect_effect},
        equipment::{Equipment, EquipmentJsonKey},
        experience::{build_exp_to_next_level, build_experience},
        inventory::{Consumable, ConsumableKind},
        loot::LootType,
        rank::Rank,
        rounds_information::AmountType,
    },
    common::{
        constants::{character_const::ULTIMATE_LEVEL, paths_const::*, stats_const::*},
        log_data::{
            LogData,
            const_colors::{DARK_RED, LIGHT_BLUE, LIGHT_GREEN},
        },
    },
    server::{
        end_of_scenario::{EndOfScenario, LevelUp},
        game_paths::GamePaths,
        game_state::{GameState, GameStatus},
        players_manager::{DodgeInfo, GameAtkEffect, PlayerManager},
        scenario::{Scenario, ScenarioState},
    },
    utils,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

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
    pub turn_nb: usize,
    pub round_nb: usize,
    /// True when the finishing blow was delivered by a damage-over-time effect (regen tick), not the direct attack.
    pub is_dot_kill: bool,
    /// Last attack name of the character killed by DOT (empty if not a DOT kill).
    pub dying_char_last_atk: String,
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

    /// Set active bosses from the current scenario's boss patterns.
    /// Bosses whose name matches a pattern in the current scenario are cloned and
    /// pushed into `pm.active_bosses` with a unique id_name (`"<name>_#<n>"`).
    pub fn set_active_bosses(&mut self, all_bosses: &[Character]) {
        self.current_scenario
            .boss_patterns
            .iter()
            .for_each(|(boss_name, _)| {
                if let Some(b) = all_bosses.iter().find(|b| b.db_full_name == *boss_name) {
                    let mut boss_to_push = b.clone();
                    boss_to_push.id_name = format!(
                        "{}_#{}",
                        boss_to_push.db_full_name,
                        1 + self
                            .pm
                            .get_nb_of_active_bosses_by_name(&boss_to_push.db_full_name)
                    );
                    self.pm.active_bosses.push(boss_to_push);
                } else {
                    tracing::warn!("Boss {} not found in data manager, skipping it", boss_name);
                }
            });
    }

    pub fn load_next_scenario(&mut self) -> Result<()> {
        // update current scenario state
        if let Some((_, state)) = self
            .states_scenarios
            .iter_mut()
            .find(|(name, _)| *name == &self.current_scenario.name)
        {
            *state = ScenarioState::Completed;
        }
        let current_level = self.current_scenario.level;
        // get the next scenario with the next level
        let Some(scenario) = self
            .all_scenarios
            .iter()
            .find(|s| s.level == current_level + 1)
            .cloned()
        else {
            return Err(anyhow::anyhow!(
                "No next scenario found for level {}",
                current_level + 1
            ));
        };
        // update scenario state in map
        if let Some((_, state)) = self
            .states_scenarios
            .iter_mut()
            .find(|(name, _)| *name == &scenario.name)
        {
            *state = ScenarioState::InProgress;
        }
        // update current scenario
        self.current_scenario = scenario;

        if self.current_scenario.level > 1 {
            // accumulate kills from the completed scenario before clearing
            let scenario_kills = self
                .pm
                .active_bosses
                .iter()
                .filter(|b| b.stats.is_dead().unwrap_or(false))
                .count();
            self.game_state.accumulated_kills += scenario_kills;
            // clear previous scenario
            self.game_state.clear_scenario();
            self.pm.clear_scenario();
            // set active bosses for the new scenario from the stored roster
            // do it before start new turn and after clearing a scenario
            let all_bosses = self.pm.all_bosses.clone();
            self.set_active_bosses(&all_bosses);
            let _ = self.start_new_turn();
        } else {
            // set active bosses for the new scenario from the stored roster
            let all_bosses = self.pm.all_bosses.clone();
            self.set_active_bosses(&all_bosses);
        }

        Ok(())
    }

    pub fn all_scenarios_completed(&self) -> bool {
        self.states_scenarios
            .values()
            .all(|state| *state == ScenarioState::Completed)
    }

    /// Start the game by starting a new turn
    pub fn start_game(&mut self) {
        // Start a new turn
        let _ = self.start_new_turn();
    }

    /// Process the start of a new turn:
    /// - Process the order of the players to play
    /// - Increment the turn number
    /// - Reset the round number
    ///
    /// Return a boolean to know if the new turn has been started and the logs of the new round if it is the case
    pub fn start_new_turn(&mut self) -> (bool, Vec<LogData>) {
        // For each turn now
        // Process the order of the players
        self.process_order_to_play();
        self.game_state.start_new_turn();
        self.pm.start_new_turn(self.game_state.current_turn_nb == 1);

        self.new_round()
    }

    /// Process the order of the players to play by sorting them by speed and adding the supplementary atk turns for the heroes and the bosses
    pub fn process_order_to_play(&mut self) {
        // to be improved with stats
        // one player can play several times as well in different order
        self.game_state.order_to_play.clear();

        // add heroes
        // sort by speed descending (highest speed acts first)
        self.pm
            .active_heroes
            .sort_by(|a, b| b.stats.all_stats[SPEED].cmp(&a.stats.all_stats[SPEED]));
        let mut dead_heroes = Vec::new();
        for hero in &self.pm.active_heroes {
            if !hero.stats.is_dead().unwrap_or(false) {
                self.game_state.order_to_play.push(hero.id_name.clone());
            } else {
                dead_heroes.push(hero.id_name.clone());
            }
        }
        // add dead heroes
        for name in dead_heroes {
            self.game_state.order_to_play.push(name);
        }
        // add bosses
        // sort by speed descending (highest speed acts first)
        self.pm
            .active_bosses
            .sort_by(|a, b| b.stats.all_stats[SPEED].cmp(&a.stats.all_stats[SPEED]));
        for boss in &self.pm.active_bosses {
            if !boss.stats.is_dead().unwrap_or(false) {
                self.game_state.order_to_play.push(boss.id_name.clone());
            }
        }
        // supplementary atks to be added
        let supp_rounds_heroes = self.pm.process_sup_atk_turn(CharacterKind::Hero);
        let supp_rounds_bosses = self.pm.process_sup_atk_turn(CharacterKind::Boss);
        self.game_state.order_to_play.extend(supp_rounds_heroes);
        self.game_state.order_to_play.extend(supp_rounds_bosses);
    }

    pub fn new_round(&mut self) -> (bool, Vec<LogData>) {
        self.game_state.new_round();
        // Still round to play
        if self.game_state.current_round > self.game_state.order_to_play.len() {
            return (
                false,
                vec![LogData {
                    message: "End of turn has been reached".to_string(),
                    ..Default::default()
                }],
            );
        }
        let Ok(mut logs) = self.pm.update_current_player_on_new_round(
            &self.game_state,
            &self.game_state.order_to_play[self.game_state.current_round - 1],
        ) else {
            // return the error of update_current_player
            return (
                false,
                vec![LogData {
                    message: "Error while updating current player".to_string(),
                    ..Default::default()
                }],
            );
        };

        if self.pm.current_player.stats.is_dead() == Some(true) {
            let (all_heroes_dead, all_bosses_dead) = self.pm.check_end_of_game();
            if all_heroes_dead {
                self.game_state.status = GameStatus::EndOfGame;
                return (false, logs);
            } else if all_bosses_dead {
                self.game_state.status = GameStatus::EndOfScenario;
                self.process_end_of_scenario();
                return (false, logs);
            }
            return self.new_round();
        }

        self.pm.reset_targeted_character();

        // Insert a round-separator at the front so the log sheet can group events per round
        logs.insert(
            0,
            LogData {
                message: format!(
                    "\u{1f501} Turn {} — Round {}",
                    self.game_state.current_turn_nb, self.game_state.current_round
                ),
                color: crate::common::log_data::const_colors::LIGHT_BLUE.to_owned(),
            },
        );

        (true, logs)
    }

    /// Launch an attack from the current player
    /// If atk_name is None and it is an auto round (boss), a random atk will be chosen
    /// Otherwise, if atk_name is None, no atk will be launched
    pub fn launch_attack(&mut self, atk_name: Option<&str>) -> ResultLaunchAttack {
        // is atk existing?
        let Some(atk_name) = atk_name else {
            if self.is_round_auto() {
                // check if pattern exists in scenario
                if let Some(patterns) = self
                    .current_scenario
                    .boss_patterns
                    .get(&self.pm.current_player.db_full_name)
                    .cloned()
                {
                    // fill queue from pattern on first use, then cycle
                    if self
                        .pm
                        .current_player
                        .character_rounds_info
                        .atk_pattern_queue
                        .is_empty()
                    {
                        self.pm
                            .current_player
                            .character_rounds_info
                            .atk_pattern_queue
                            .extend(patterns.iter().copied());
                    }
                    if let Some(idx) = self
                        .pm
                        .current_player
                        .character_rounds_info
                        .atk_pattern_queue
                        .pop_front()
                        && let Some((atk_name, _)) =
                            self.pm.current_player.attacks_list.get_index(idx as usize)
                    {
                        let atk_name = atk_name.clone();
                        tracing::info!(
                            "Auto attack for boss {}: {}",
                            self.pm.current_player.id_name,
                            atk_name
                        );
                        return self.launch_attack(Some(&atk_name));
                    }
                }
                // auto atk for boss
                if let Some(auto_atk_name) = AttackType::get_one_random_atk_name(
                    &self.pm.current_player.character_rounds_info.launchable_atks,
                ) {
                    tracing::info!(
                        "Auto attack for boss {}: {}",
                        self.pm.current_player.id_name,
                        auto_atk_name
                    );
                    return self.launch_attack(Some(&auto_atk_name));
                }
            }

            return self.process_no_atk_launched();
        };
        // output
        let mut new_game_atk_effects: Vec<GameAtkEffect> = vec![];
        // update action done in round
        self.pm
            .current_player
            .character_rounds_info
            .actions_done_in_round += 1;
        // get all players
        let all_players = self.pm.get_all_active_id_names();
        // get atk
        let atk_list = self.pm.current_player.attacks_list.clone();
        let atk = match atk_list.get(atk_name) {
            Some(atk) => atk.clone(),
            None => {
                // unknown atk
                tracing::error!(
                    "Error: attack {} not found for player {}",
                    atk_name,
                    self.pm.current_player.id_name
                );
                return self.process_no_atk_launched();
            }
        };

        // can be launched
        // process cost
        self.pm.current_player.process_atk_cost(atk_name);

        // is dodging ?
        self.pm.process_all_dodging(
            &all_players,
            self.pm.current_player.attacks_list[atk_name].level,
            &self.pm.current_player.clone().kind,
        );

        // critical strike
        let is_crit = match self.pm.current_player.process_critical_strike(atk_name) {
            Ok(is_crit) => is_crit,
            Err(e) => {
                tracing::error!(
                    "Error while processing critical strike for player {}: {}",
                    self.pm.current_player.id_name,
                    e
                );
                false
            }
        };
        // process boss target
        self.pm.process_boss_target();

        // ProcessAtk
        let all_effects_param =
            match self
                .pm
                .current_player
                .process_atk(&self.game_state, is_crit, &atk)
            {
                Ok(effects) => effects,
                Err(e) => {
                    tracing::error!(
                        "Error while processing attack {} for player {}: {}",
                        atk_name,
                        self.pm.current_player.id_name,
                        e
                    );
                    vec![]
                }
            };
        // apply effect param on targets
        let launcher_stats = self.pm.current_player.stats.clone();
        let id_name = self.pm.current_player.id_name.clone();
        let kind = self.pm.current_player.kind.clone();
        let mut all_dodging = vec![];
        let launcher_info = LauncherAtkInfo {
            id_name: id_name.clone(),
            kind,
            stats: launcher_stats,
            atk_type: atk.clone(),
        };

        let mut new_gaes: Vec<GameAtkEffect> = Vec::new();
        for processed_effect in &all_effects_param {
            for target_id_name in &all_players {
                let mut gae: Option<GameAtkEffect> = None;
                let mut all_di: Option<Vec<DodgeInfo>> = None;
                if id_name == *target_id_name {
                    (gae, all_di) = self.pm.current_player.is_receiving_atk(
                        processed_effect,
                        &self.game_state,
                        is_crit,
                        &launcher_info,
                    );
                    tracing::trace!(
                        "Effect outcome for self target {}: {:?}",
                        target_id_name,
                        gae
                    );
                } else if let Some(c) = self.pm.get_mut_active_character(target_id_name) {
                    (gae, all_di) = c.is_receiving_atk(
                        processed_effect,
                        &self.game_state,
                        is_crit,
                        &launcher_info,
                    );
                    tracing::trace!("Effect outcome for target {}: {:?}", target_id_name, gae);
                } else {
                    tracing::trace!("Effect outcome for unknown target {}", target_id_name);
                }
                if let Some(mut di) = all_di {
                    all_dodging.append(&mut di);
                };
                if let Some(new_gae) = gae {
                    new_game_atk_effects.push(new_gae.clone());
                    new_gaes.push(new_gae.clone());
                };
            }
        }

        // other function
        // Apply total aggro generated by all effects to the launcher so that boss
        // target-selection correctly tracks which hero has been most active.
        let total_aggro: u64 = new_gaes
            .iter()
            .map(|g| g.effect_outcome.aggro_generated)
            .sum();
        if total_aggro > 0 {
            self.pm.current_player.process_aggro(
                0,
                total_aggro as i64,
                self.game_state.current_turn_nb,
            );
        }

        // Accumulate the damage transmitted by the launcher this turn so effects that
        // depend on prior damage dealt (e.g. ConditionDamagePrevTurn) can read it back.
        // real_amount_tx is negative on a damaging HP effect; store the magnitude.
        let total_damage_tx: i64 = new_gaes
            .iter()
            .filter(|g| {
                g.processed_effect_param
                    .input_effect_param
                    .buffer
                    .stats_name
                    == HP
                    && g.effect_outcome.real_amount_tx < 0
            })
            .map(|g| g.effect_outcome.real_amount_tx.abs())
            .sum();
        if total_damage_tx > 0
            && let Some(map) = self
                .pm
                .current_player
                .character_rounds_info
                .tx_rx
                .get_mut(AmountType::DamageTx as usize)
        {
            *map.entry(self.game_state.current_turn_nb as u64)
                .or_insert(0) += total_damage_tx;
        }

        // Fire IsDamageTxHealNeedyAlly passive immediately after damage is dealt.
        let passive_logs = if !self.pm.current_player.is_boss_atk() && total_damage_tx > 0 {
            self.pm
                .apply_damage_tx_heal_passive(&id_name.clone(), total_damage_tx)
        } else {
            Vec::new()
        };

        // update tx rx
        if is_crit
            && let Some(map) = self
                .pm
                .current_player
                .character_rounds_info
                .tx_rx
                .get_mut(AmountType::CriticalStrike as usize)
        {
            *map.entry(self.game_state.current_turn_nb as u64)
                .or_insert(0) += 1;
        }
        // end of buf

        // new effects to add on the different players
        // RemoveTerminatedEffectsOnPlayer which last only that turn

        // check who died
        self.pm.process_died_players().unwrap_or_else(|e| {
            tracing::error!("Error while processing died players: {}", e);
        });
        // TODO if boss died -> loot

        // record the attack on the current player so we can surface it as the dying char's last move
        self.pm.current_player.last_atk_name = atk_name.to_string();

        // update active character for cost atk and buf received.
        self.pm
            .modify_active_character(&self.pm.current_player.id_name.clone());

        // process stats
        self.game_state.process_game_stats(
            &new_gaes,
            &self.pm.current_player.id_name.clone(),
            atk_name,
        );

        // snapshot: were all bosses (or heroes) already dead before the end-of-round processing?
        let bosses_dead_before_eor = self.pm.check_end_of_game().1;

        // process end of attack
        let mut logs_atk = self.build_logs_atk(&all_dodging, &new_game_atk_effects, is_crit);
        logs_atk.extend(passive_logs);
        let mut result_attack = ResultLaunchAttack {
            launcher_id_name: self.pm.current_player.id_name.clone(),
            atk_name: atk_name.to_string(),
            is_crit,
            new_game_atk_effects: new_game_atk_effects.clone(),
            all_dodging: all_dodging.clone(),
            is_boss_atk: self.pm.current_player.is_boss_atk(),
            logs_end_of_round: Vec::new(),
            logs_atk,
            turn_nb: self.game_state.current_turn_nb,
            round_nb: self.game_state.current_round,
            is_dot_kill: false,
            dying_char_last_atk: String::new(),
        };

        // eval next step of the game
        result_attack.logs_end_of_round = self.eval_end_of_round(result_attack.logs_atk.clone());

        // if bosses were alive before end-of-round but scenario ended during it, a DOT finished them
        if !bosses_dead_before_eor && self.game_state.status == GameStatus::EndOfScenario {
            result_attack.is_dot_kill = true;
            if let Some(dead_boss) = self
                .pm
                .active_bosses
                .iter()
                .find(|b| b.stats.is_dead().unwrap_or(false))
            {
                result_attack.dying_char_last_atk = dead_boss.last_atk_name.clone();
            }
        }

        // update game state with the result of the attack
        self.game_state.last_result_atk = result_attack.clone();

        result_attack
    }

    /// Process end-of-scenario rewards for every hero:
    /// - Add loot items matching the hero's class (equipment checked against the equipment database,
    ///   consumables and currency added directly)
    /// - Add experience gained from all defeated bosses and level up (with stat update) as needed
    /// - Automatically use all consumables in inventory (potions restore HP)
    ///   Process end of scenario struct to be sent to the frontend with the rewards and the level up info
    pub fn process_end_of_scenario(&mut self) {
        // Total exp: sum from all bosses
        let total_exp: u64 = self
            .pm
            .active_bosses
            .iter()
            .map(|boss| build_experience(&boss.rank, boss.level))
            .sum();

        let loots = self.current_scenario.loots.clone();
        let equipment_table_flat: Vec<Equipment> = self
            .pm
            .equipment_table
            .values()
            .flatten()
            .cloned()
            .collect();

        // prepare end of scenario
        self.end_of_scenario.scenario_level = self.current_scenario.level;
        self.end_of_scenario.characters_levelup.clear();
        self.pm.active_heroes.iter().for_each(|hero| {
            self.end_of_scenario.characters_levelup.push(LevelUp {
                character_id_name: hero.id_name.clone(),
                new_level: hero.level,
                old_level: hero.level,
            });
        });

        for i in 0..self.pm.active_heroes.len() {
            let hero_class = self.pm.active_heroes[i].class.clone();

            // Add loot according to class
            for loot in &loots {
                let class_matches =
                    loot.classes.contains(&hero_class) || loot.classes.contains(&Class::Standard);
                if !class_matches {
                    continue;
                }
                match &loot.kind {
                    LootType::Equipment => {
                        if let Some(equipment) = equipment_table_flat
                            .iter()
                            .find(|e| e.unique_name == loot.name)
                            .cloned()
                        {
                            self.pm.active_heroes[i]
                                .inventory
                                .add_equipment(&equipment, false);
                        } else {
                            tracing::warn!(
                                "Equipment '{}' not found in equipment database",
                                loot.name
                            );
                        }
                    }
                    LootType::Consumable => {
                        // Consumables go to the shared party bag (handled below).
                    }
                    LootType::Currency => {
                        self.pm.active_heroes[i].inventory.money += loot.level as u64;
                    }
                    LootType::Material => {}
                }
            }

            // Add experience and level up if needed
            self.pm.active_heroes[i].character_rounds_info.exp += total_exp;
            while self.pm.active_heroes[i].character_rounds_info.exp
                >= self.pm.active_heroes[i]
                    .character_rounds_info
                    .exp_to_next_level
                && self.pm.active_heroes[i].level < ULTIMATE_LEVEL
            {
                self.pm.active_heroes[i].character_rounds_info.exp -= self.pm.active_heroes[i]
                    .character_rounds_info
                    .exp_to_next_level;
                self.pm.active_heroes[i].level += 1;
                self.pm.active_heroes[i].stats.update_stats_to_next_level();
                // Recompute the threshold for the new level
                self.pm.active_heroes[i]
                    .character_rounds_info
                    .exp_to_next_level = build_exp_to_next_level(
                    &self.pm.active_heroes[i].rank,
                    &self.pm.active_heroes[i].class,
                    self.pm.active_heroes[i].level,
                );
                // update end of scenario
                if let Some(level_up) = self
                    .end_of_scenario
                    .characters_levelup
                    .iter_mut()
                    .find(|lu| lu.character_id_name == self.pm.active_heroes[i].id_name)
                {
                    level_up.new_level = self.pm.active_heroes[i].level;
                }
            }
        }

        // Add consumable loot to the shared party bag (once per loot item, not per hero).
        for loot in &loots {
            if loot.kind != LootType::Consumable {
                continue;
            }
            let any_hero_matches = self.pm.active_heroes.iter().any(|hero| {
                loot.classes.contains(&hero.class) || loot.classes.contains(&Class::Standard)
            });
            if any_hero_matches {
                let effects = Self::build_consumable_effects(&loot.name, &loot.rank);
                self.pm.party_consumables.push(Consumable {
                    name: loot.name.clone(),
                    effects,
                    consumable_kind: ConsumableKind::Potion,
                    rank: loot.rank.clone(),
                });
            }
        }
    }

    fn build_consumable_effects(
        name: &str,
        rank: &Rank,
    ) -> Vec<crate::character_mod::effect::EffectParam> {
        use crate::common::constants::stats_const::{BERSERK, MANA, VIGOR};
        match name {
            "potion of resurrection" => {
                let value = match rank {
                    Rank::Common => 20,
                    Rank::Intermediate => 50,
                    Rank::Advanced => 100,
                };
                vec![build_resurrect_effect(value)]
            }
            "mana potion" => {
                let value = match rank {
                    Rank::Common => 30,
                    Rank::Intermediate => 70,
                    Rank::Advanced => 150,
                };
                vec![build_energy_effect(MANA, value)]
            }
            "vigor potion" => {
                let value = match rank {
                    Rank::Common => 30,
                    Rank::Intermediate => 70,
                    Rank::Advanced => 150,
                };
                vec![build_energy_effect(VIGOR, value)]
            }
            "berserk potion" => {
                let value = match rank {
                    Rank::Common => 30,
                    Rank::Intermediate => 70,
                    Rank::Advanced => 150,
                };
                vec![build_energy_effect(BERSERK, value)]
            }
            _ => {
                let value = match rank {
                    Rank::Common => 20,
                    Rank::Intermediate => 60,
                    Rank::Advanced => 120,
                };
                vec![build_hp_effect(value, false)]
            }
        }
    }

    fn process_no_atk_launched(&mut self) -> ResultLaunchAttack {
        // no atk launched
        // update action done in round
        self.pm
            .current_player
            .character_rounds_info
            .actions_done_in_round += 1;
        let logs_atk = vec![LogData {
            message: "No attack launched".to_string(),
            color: DARK_RED.to_string(),
        }];
        // eval next step of the game
        let logs_end_of_round = self.eval_end_of_round(logs_atk.clone());
        ResultLaunchAttack {
            launcher_id_name: self.pm.current_player.id_name.clone(),
            is_boss_atk: self.pm.current_player.is_boss_atk(),
            logs_end_of_round,
            logs_atk,
            turn_nb: self.game_state.current_turn_nb,
            round_nb: self.game_state.current_round,
            ..Default::default()
        }
    }

    /// Evaluate the end of the round by checking if the game is finished,
    ///  if a new round should start or if a new turn should start,
    ///  and return the logs to display for the new round if it is the case
    fn eval_end_of_round(&mut self, logs_atk: Vec<LogData>) -> Vec<LogData> {
        let mut output_logs = vec![];
        let (all_heroes_dead, all_bosses_dead) = self.pm.check_end_of_game();
        if all_heroes_dead {
            self.game_state.status = GameStatus::EndOfGame;
        } else if all_bosses_dead {
            self.game_state.status = GameStatus::EndOfScenario;
            self.process_end_of_scenario();
        } else {
            let (is_new_round, logs) = self.new_round();
            output_logs.extend(logs);
            // new_round may have triggered EndOfScenario/EndOfGame (e.g. boss killed by DOT)
            if matches!(
                self.game_state.status,
                GameStatus::EndOfScenario | GameStatus::EndOfGame
            ) {
                // Status already set inside new_round; nothing more to do
            } else if is_new_round {
                self.game_state.status = GameStatus::StartRound;
            } else {
                let (is_new_turn, logs) = self.start_new_turn();
                output_logs.extend(logs);
                if matches!(
                    self.game_state.status,
                    GameStatus::EndOfScenario | GameStatus::EndOfGame
                ) {
                    // Status set inside start_new_turn via new_round
                } else if is_new_turn {
                    self.game_state.status = GameStatus::StartRound;
                } else {
                    self.game_state.status = GameStatus::EndOfGame;
                }
            }
        }

        self.logs.extend(output_logs.clone());
        self.logs.extend(logs_atk.clone());

        output_logs
    }

    pub fn build_logs_atk(
        &self,
        all_dodging: &Vec<DodgeInfo>,
        all_gae: &Vec<GameAtkEffect>,
        is_crit: bool,
    ) -> Vec<LogData> {
        let mut logs: Vec<LogData> = vec![];
        // dodging and blocking info
        for d in all_dodging {
            tracing::debug!("Dodge info for {}: {:?}", d.name, d);
            if d.is_dodging {
                logs.push(LogData {
                    message: format!("{} is dodging", d.name),
                    color: LIGHT_BLUE.to_string(),
                });
            } else if d.is_blocking {
                logs.push(LogData {
                    message: format!("{} is blocking", d.name),
                    color: LIGHT_GREEN.to_string(),
                });
            }
        }
        // logs for the atk
        if !all_gae.is_empty() {
            // Derive attacker + attack name from the first gae
            let attacker = &self.pm.current_player.id_name;
            let atk_name = all_gae
                .first()
                .map(|g| g.atk_type.name.as_str())
                .unwrap_or("?");
            logs.push(LogData {
                message: utils::format_string_with_timestamp(&format!(
                    "⚔ {} uses {}",
                    attacker, atk_name
                )),
                color: "".to_string(),
            });
            if is_crit {
                logs.push(LogData {
                    message: "💥 Critical strike!".to_string(),
                    color: DARK_RED.to_string(),
                });
            }

            for gae in all_gae {
                // log for the processed effect param
                if !gae.processed_effect_param.log.message.is_empty() {
                    logs.push(gae.processed_effect_param.log.clone());
                }
                // log for the effect outcome
                let mut colortext = LIGHT_GREEN;
                let is_hp_effect = gae
                    .processed_effect_param
                    .input_effect_param
                    .buffer
                    .stats_name
                    == HP;
                let is_damage = is_hp_effect
                    && (gae.effect_outcome.real_amount_tx < 0
                        || gae.effect_outcome.full_amount_tx < 0);
                if is_damage {
                    colortext = DARK_RED;
                }
                if gae.processed_effect_param.input_effect_param.buffer.kind
                    == BufKinds::CooldownTurnsNumber
                {
                    logs.push(LogData {
                        color: colortext.to_string(),
                        message: format!(
                            "{} ← Cooldown for {} turns",
                            gae.effect_outcome.target_id_name,
                            gae.processed_effect_param.input_effect_param.buffer.value
                        ),
                    });
                } else if is_hp_effect {
                    let pre = gae.effect_outcome.pre_armor_amount_tx;
                    let full = gae.effect_outcome.full_amount_tx;
                    let real = gae.effect_outcome.real_amount_tx;
                    let msg = if is_damage {
                        if pre == real {
                            format!("{} ← {} HP", gae.effect_outcome.target_id_name, real)
                        } else {
                            format!(
                                "{} ← {} HP (full: {}, real: {})",
                                gae.effect_outcome.target_id_name, real, pre, real
                            )
                        }
                    } else if full == real {
                        format!(
                            "{} ← {} HP ({})",
                            gae.effect_outcome.target_id_name,
                            real,
                            gae.processed_effect_param.input_effect_param.buffer.kind
                        )
                    } else {
                        format!(
                            "{} ← {} HP (full: {}, real: {})",
                            gae.effect_outcome.target_id_name, real, full, real
                        )
                    };
                    logs.push(LogData {
                        color: colortext.to_string(),
                        message: msg,
                    });
                } else if !gae
                    .processed_effect_param
                    .input_effect_param
                    .buffer
                    .stats_name
                    .is_empty()
                {
                    logs.push(LogData {
                        color: colortext.to_string(),
                        message: format!(
                            "{} ← {} {} ({})",
                            gae.effect_outcome.target_id_name,
                            gae.effect_outcome.full_amount_tx,
                            gae.processed_effect_param
                                .input_effect_param
                                .buffer
                                .stats_name,
                            gae.processed_effect_param.input_effect_param.buffer.kind
                        ),
                    });
                }
            }
        }
        logs
    }

    /// Check if it is the turn to a boss to play
    /// HMI function
    pub fn is_round_auto(&self) -> bool {
        if self.game_state.current_round as i64 > 0
            && self.game_state.current_round as i64 - 1 < self.game_state.order_to_play.len() as i64
        {
            let name = self.game_state.order_to_play[self.game_state.current_round - 1].clone();
            if let Some(c) = self.pm.get_active_character(&name) {
                return c.kind == CharacterKind::Boss;
            }
        }

        false
    }

    /// Process the number of bosses that are attacking in a row in the current round, to know if it is the case to add a log for the new round with the info of the boss attack
    /// boss should not be dead to be counted
    /// used by dx-rpg
    pub fn process_nb_bosses_atk_in_a_row(&self) -> i64 {
        let mut count = 0;

        if self.game_state.current_round as i64 > 0
            && self.game_state.current_round as i64 - 1 < self.game_state.order_to_play.len() as i64
        {
            // Start from current_round and go to the end
            for i in self.game_state.current_round - 1..self.game_state.order_to_play.len() {
                let name = &self.game_state.order_to_play[i];

                if let Some(c) = self.pm.get_active_character(name) {
                    if c.kind == CharacterKind::Boss && c.stats.is_dead() != Some(true) {
                        count += 1;
                    } else {
                        break; // Stop counting when a non-Boss is found
                    }
                } else {
                    break; // Stop counting if character doesn't exist
                }
            }
        }

        count
    }
}

#[cfg(test)]
mod tests {
    use crate::character_mod::attack_type::AttackType;
    use crate::character_mod::buffers::{BufKinds, Buffer};
    use crate::character_mod::character::CharacterKind;
    use crate::character_mod::class::Class;
    use crate::character_mod::rank::Rank;
    use crate::common::constants::attak_const::COEFF_CRIT_DMG;
    use crate::common::constants::streak_breaker_const::STREAK_BREAKER_ADVANCED;
    use crate::common::log_data::const_colors::DARK_RED;
    use crate::server::game_manager::LogData;
    use crate::server::game_state::GameStatus;
    use crate::testing::testing_all_characters::{
        self, testing_game_manager, testing_test_ally1_vs_test_boss1,
    };
    use crate::{
        common::constants::{character_const::SPEED_THRESHOLD, stats_const::*},
        testing::testing_atk::*,
    };

    #[test]
    fn unit_process_order_to_play() {
        let mut gm = testing_game_manager();
        let old_speed = gm
            .pm
            .get_mut_active_hero_character("test_#1")
            .cloned()
            .unwrap()
            .stats
            .all_stats[SPEED]
            .clone();
        gm.process_order_to_play();
        let new_speed = gm
            .pm
            .get_mut_active_hero_character("test_#1")
            .cloned()
            .unwrap()
            .stats
            .all_stats[SPEED]
            .clone();
        // only one supplementary attack per turn: test2_#1 (fastest hero, speed 312) qualifies;
        // test_#1 (212) is skipped because process_sup_atk_turn returns after the first hit.
        assert_eq!(gm.game_state.order_to_play.len(), 5);
        // descending speed sort: test2_#1 (312) before test_#1 (212)
        assert_eq!(gm.game_state.order_to_play[0], "test2_#1");
        assert_eq!(gm.game_state.order_to_play[1], "test_#1");
        // descending speed sort: boss2 (15) before boss1 (11)
        assert_eq!(gm.game_state.order_to_play[2], "test_boss2_#1");
        assert_eq!(gm.game_state.order_to_play[3], "test_boss1_#1");
        // only test2_#1 gets the supplementary slot
        assert_eq!(gm.game_state.order_to_play[4], "test2_#1");
        // test_#1 speed is unchanged (it did NOT get the supplementary slot)
        assert_eq!(old_speed.current, new_speed.current);
        assert_eq!(old_speed.max, new_speed.max);
        assert_eq!(old_speed.max_raw, new_speed.max_raw);
        assert_eq!(old_speed.current_raw, new_speed.current_raw);
        // test2_#1 had its speed reset (312 - SPEED_THRESHOLD = 212)
        let new_test2_speed = gm
            .pm
            .get_mut_active_hero_character("test2_#1")
            .cloned()
            .unwrap()
            .stats
            .all_stats[SPEED]
            .clone();
        assert_eq!(312 - SPEED_THRESHOLD, new_test2_speed.current);
        // one hero player is dead — use name-based kill so the index stays stable after sort
        gm.pm
            .get_mut_active_hero_character("test_#1")
            .unwrap()
            .stats
            .all_stats[HP]
            .current = 0;
        gm.process_order_to_play();
        assert_eq!(gm.game_state.order_to_play.len(), 5);
        assert_eq!(gm.game_state.order_to_play[0], "test2_#1");
        assert_eq!(gm.game_state.order_to_play[1], "test_#1");
        assert_eq!(gm.game_state.order_to_play[2], "test_boss2_#1");
        assert_eq!(gm.game_state.order_to_play[3], "test_boss1_#1");
        assert_eq!(gm.game_state.order_to_play[4], "test2_#1");
        // boss is dead — use name-based kill; descending sort puts boss2 at index 0
        gm.pm
            .get_mut_active_boss_character("test_boss1_#1")
            .unwrap()
            .stats
            .all_stats[HP]
            .current = 0;
        gm.process_order_to_play();
        assert_eq!(gm.game_state.order_to_play.len(), 4);
        assert_eq!(gm.game_state.order_to_play[0], "test2_#1");
        assert_eq!(gm.game_state.order_to_play[1], "test_#1");
        assert_eq!(gm.game_state.order_to_play[2], "test_boss2_#1");
        assert_eq!(gm.game_state.order_to_play[3], "test2_#1");
    }

    #[test]
    fn unit_add_sup_atk_turn() {
        let mut gm = testing_all_characters::testing_game_manager();
        let hero = gm.pm.active_heroes.first_mut().unwrap();
        hero.stats.all_stats.get_mut(SPEED).unwrap().current = 300;
        let boss = gm.pm.active_bosses.first_mut().unwrap();
        boss.stats.all_stats.get_mut(SPEED).unwrap().current = 10;
        let result = gm.pm.process_sup_atk_turn(CharacterKind::Hero);
        // only one supplementary attack per call — the first qualifying hero
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn unit_new_round() {
        let mut gm = testing_all_characters::testing_game_manager();
        let result = gm.start_new_turn();
        assert!(result.0);
        assert_eq!(gm.game_state.current_round, 1);

        // test current player -test- is dead - round for boss is starting
        gm.game_state.current_round = 0;
        gm.pm.active_heroes[0].stats.all_stats[HP].current = 0;
        let result = gm.new_round();
        assert!(result.0);
        assert_eq!(gm.game_state.current_round, 2);
        // test current round > table order to play
        gm.game_state.current_round = 1000;
        let result = gm.new_round();
        assert!(!result.0);
        // character name in orderToplay list is not a player
        gm.game_state.order_to_play.clear();
        gm.game_state.order_to_play.push("unknown".to_owned());
        gm.game_state.current_round = 0;
        let result = gm.new_round();
        assert!(!result.0);
    }

    #[test]
    fn unit_launch_attack_none_atk_hero() {
        let (mut gm, _hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();

        // test unknown atk
        let ra = gm.launch_attack(None);
        assert_eq!(
            ra.logs_atk,
            vec![LogData {
                message: "No attack launched".to_string(),
                color: DARK_RED.to_string(),
            }]
        );
    }

    #[test]
    fn unit_launch_attack_simple_atk_vigor() {
        let (mut gm, hero_launcher_id_name, target_id_name) = testing_test_ally1_vs_test_boss1();

        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .stats
            .all_stats[DODGE]
            .current = 0;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_boss = gm
            .pm
            .get_active_boss_character(&target_id_name)
            .unwrap()
            .clone();
        let old_hp_boss = old_boss.stats.all_stats[HP].current;
        let old_vigor_hero = gm.pm.current_player.stats.all_stats[VIGOR].current;

        // test normal atk
        // set target
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;
        let ra = gm.launch_attack(Some("SimpleAtk"));

        assert_eq!(1, ra.new_game_atk_effects.len());
        assert!(ra.all_dodging.is_empty());
        assert!(!ra.logs_atk.is_empty());
        // not dead boss : end of game
        assert!(gm.game_state.status != GameStatus::EndOfGame);
        // raw = -35(atk) - total_phy_pow, protection = ARMOR_FACTOR/(ARMOR_FACTOR + total_phy_armor)
        let hero_total_pow = gm.pm.current_player.stats.get_power_stat(false);
        let boss_total_armor = old_boss.stats.get_armor_stat(false);
        let raw_dmg = (35 + hero_total_pow) as f64;
        let protection =
            AttackType::ARMOR_FACTOR / (AttackType::ARMOR_FACTOR + boss_total_armor as f64);
        let atk_amount = (raw_dmg * protection).round() as i64;
        assert_eq!(
            std::cmp::max(0, old_hp_boss as i64 - atk_amount) as u64,
            gm.pm
                .get_active_boss_character(&target_id_name)
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        // cost: 9 % of vigor 200 = 18
        assert_eq!(
            old_vigor_hero - 18,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[VIGOR]
                .current
        );
    }

    #[test]
    fn unit_launch_attack_simple_atk_vigor_on_dodging_ennemy() {
        let (mut gm, hero_launcher_id_name, target_id_name) = testing_test_ally1_vs_test_boss1();

        // # case 2 dmg on individual ennemy
        // dodging of boss — guaranteed via streak-breaker
        // no critical of current player
        // atk cost is even processed

        // Use streak-breaker to guarantee the boss dodge: Advanced rank at level 5,
        // drought counter at the threshold ensures the next dodge is certain.
        {
            let boss = gm
                .pm
                .get_mut_active_boss_character(&target_id_name)
                .unwrap();
            boss.rank = Rank::Advanced;
            boss.level = 5;
            boss.stats.all_stats[DODGE].current = 0; // softcap = 0%, streak-breaker fires
            boss.character_rounds_info.dodge_drought_counter = STREAK_BREAKER_ADVANCED;
            boss.character_rounds_info.is_current_target = true;
        }
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        // Disable the NextHealAtkIsCrit passive to ensure no crit on this non-heal atk
        if let Some(buf) = gm
            .pm
            .current_player
            .character_rounds_info
            .get_mut_buffer_by_type(&BufKinds::NextHealAtkIsCrit)
        {
            buf.is_passive_enabled = false;
        }
        let old_hp_boss = gm
            .pm
            .get_active_boss_character(&target_id_name)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let old_vigor_hero = gm.pm.current_player.stats.all_stats[VIGOR].current;
        gm.launch_attack(Some("SimpleAtk"));
        // not dead boss : end of game
        assert!(gm.game_state.status != GameStatus::EndOfGame);
        assert_eq!(
            old_hp_boss,
            gm.pm
                .get_active_boss_character(&target_id_name)
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        // 9% of 200 (total vigor)
        assert_eq!(
            old_vigor_hero - 18,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[VIGOR]
                .current
        );
    }

    #[test]
    fn unit_launch_attack_simple_atk_vigor_critical() {
        let (mut gm, hero_launcher_id_name, target_id_name) = testing_test_ally1_vs_test_boss1();

        // # case 3 dmg on individual ennemy
        // No dodging of boss
        // critical of current player — guaranteed via streak-breaker
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .stats
            .all_stats[DODGE]
            .current = 0;
        // Use Advanced rank + level 5 so the streak-breaker activates at threshold 5,
        // then pre-set the drought counter to the threshold to guarantee a crit.
        gm.pm.current_player.rank = Rank::Advanced;
        gm.pm.current_player.level = 5;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        gm.pm
            .current_player
            .character_rounds_info
            .crit_drought_counter = STREAK_BREAKER_ADVANCED;
        let old_boss = gm
            .pm
            .get_active_boss_character(&target_id_name)
            .unwrap()
            .clone();
        let old_hp_boss = old_boss.stats.all_stats[HP].current;
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;
        let old_vigor_hero = gm.pm.current_player.stats.all_stats[VIGOR].current;
        gm.launch_attack(Some("SimpleAtk"));
        // 1 dead boss : end of game
        assert!(gm.game_state.status != GameStatus::EndOfGame); // still one boss
        // raw = -35(atk) - total_phy_pow; armor and crit applied in sequence (rounded each step)
        let hero_total_pow = gm.pm.current_player.stats.get_power_stat(false);
        let boss_total_armor = old_boss.stats.get_armor_stat(false);
        let raw_dmg = (35 + hero_total_pow) as f64;
        let protection =
            AttackType::ARMOR_FACTOR / (AttackType::ARMOR_FACTOR + boss_total_armor as f64);
        let effective = (raw_dmg * protection).round() as i64;
        let atk_amount = (effective as f64 * COEFF_CRIT_DMG).round() as i64;
        assert_eq!(
            std::cmp::max(0, old_hp_boss as i64 - atk_amount) as u64,
            gm.pm
                .get_active_boss_character(&target_id_name)
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        // 9% of 200 (total vigor)
        assert_eq!(
            old_vigor_hero - 18,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[VIGOR]
                .current
        );
    }

    #[test]
    fn unit_launch_attack_simple_atk_on_blocking_boss() {
        let (mut gm, hero_launcher_id_name, target_id_name) = testing_test_ally1_vs_test_boss1();

        // # case 4 dmg on individual ennemy
        // No dodging of boss
        // Blocking — guaranteed via streak-breaker
        // No critical of current player
        //
        // A Berserker "dodges" by blocking, but its block chance is the softcapped dodge
        // stat, which can never reach 100%, so relying on the dice roll would be flaky.
        // A Berserker also has no default dodge streak-breaker, so set a StreakBreakerDodge
        // buffer and push the drought counter to its threshold to force a deterministic block.
        {
            let boss = gm
                .pm
                .get_mut_active_boss_character(&target_id_name)
                .unwrap();
            boss.class = Class::Berserker;
            boss.stats.all_stats[DODGE].current = 0;
            boss.character_rounds_info.update_buffer(&Buffer {
                is_passive_enabled: false,
                is_passive: false,
                value: 1,
                is_percent: false,
                stats_name: String::new(),
                kind: BufKinds::StreakBreakerDodge,
            });
            boss.character_rounds_info.dodge_drought_counter = 1;
        }
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_boss = gm
            .pm
            .get_active_boss_character(&target_id_name)
            .unwrap()
            .clone();
        let old_hp_boss = old_boss.stats.all_stats[HP].current;
        let old_vigor_hero = gm.pm.current_player.stats.all_stats[MANA].current;
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;
        gm.launch_attack(Some("SimpleAtk"));
        // not dead boss : end of game
        assert!(gm.game_state.status != GameStatus::EndOfGame);
        // raw = -35(atk) - total_phy_pow; 10% of effective damage passes through on block
        let hero_total_pow = gm.pm.current_player.stats.get_power_stat(false);
        let boss_total_armor = old_boss.stats.get_armor_stat(false);
        let raw_dmg = (35 + hero_total_pow) as f64;
        let protection =
            AttackType::ARMOR_FACTOR / (AttackType::ARMOR_FACTOR + boss_total_armor as f64);
        let effective = (raw_dmg * protection).round() as i64;
        let blocking = 10 * effective / 100;
        assert_eq!(
            (old_hp_boss as i64 - blocking) as u64,
            gm.pm
                .get_active_boss_character(&target_id_name)
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        // 9% of 200 (total vigor)
        assert_eq!(
            old_vigor_hero - 18,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[VIGOR]
                .current
        );
    }

    #[test]
    fn unit_launch_attack_atk_heal1_zone() {
        // Zone = Tous les heroes
        let (mut gm, hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();

        // # case 5 up and change on zone ally
        // ally 1 speed > ally 2 speed
        // no critical strike
        let atk = build_atk_heal1_zone();
        gm.pm
            .current_player
            .attacks_list
            .insert(atk.name.clone(), atk.clone());
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        // Disable the NextHealAtkIsCrit passive (loaded from test JSON) so this
        // heal attack is not treated as a crit.
        if let Some(buf) = gm
            .pm
            .current_player
            .character_rounds_info
            .get_mut_buffer_by_type(&BufKinds::NextHealAtkIsCrit)
        {
            buf.is_passive_enabled = false;
        }
        let old_hp_test2 = gm
            .pm
            .get_active_hero_character("test2_#1")
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let old_mana_launcher = gm.pm.current_player.stats.all_stats[MANA].current;
        gm.launch_attack(Some(&atk.clone().name));
        assert!(gm.game_state.status != GameStatus::EndOfGame);
        // + 30  of max HP:135 = 40
        assert_eq!(
            old_hp_test2 + 40,
            gm.pm
                .get_active_hero_character("test2_#1")
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        // -10% of mana max (see effect param of the atk)
        assert_eq!(
            old_mana_launcher - (0.1 * old_mana_launcher as f64) as u64,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[MANA]
                .current
        ); // 10% of 200 (total mana)
    }

    #[test]
    fn unit_launch_attack_case_eclat_despoir() {
        let (mut gm, hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();
        // no crit
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        // Disable the NextHealAtkIsCrit passive (loaded from test JSON) so this
        // heal attack is not treated as a crit.
        if let Some(buf) = gm
            .pm
            .current_player
            .character_rounds_info
            .get_mut_buffer_by_type(&BufKinds::NextHealAtkIsCrit)
        {
            buf.is_passive_enabled = false;
        }
        let old_hp_test = gm
            .pm
            .get_active_hero_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let old_mag_pow_test = gm
            .pm
            .get_active_hero_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[MAGICAL_POWER]
            .max;
        let old_phy_pow_test = gm
            .pm
            .get_active_hero_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[PHYSICAL_POWER]
            .max;
        let old_hp_test2 = gm
            .pm
            .get_active_hero_character("test2_#1")
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let old_mag_pow_test2 = gm
            .pm
            .get_active_hero_character("test2_#1")
            .unwrap()
            .stats
            .all_stats[MAGICAL_POWER]
            .max;
        let old_phy_pow_test2 = gm
            .pm
            .get_active_hero_character("test2_#1")
            .unwrap()
            .stats
            .all_stats[PHYSICAL_POWER]
            .max;
        let old_mana_launcher = gm.pm.current_player.stats.all_stats[MANA].current;
        gm.launch_attack(Some("Eclat d'espoir"));
        assert!(gm.game_state.status != GameStatus::EndOfGame);
        // "up-current-stat-by-percentage"
        // + 30 % of max HP:135 = 40.5 + NextAtkHealIsCrit x2 = 80 on test2 and test1
        assert_eq!(
            old_hp_test2 + 40,
            gm.pm
                .get_active_hero_character("test2_#1")
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        assert_eq!(
            old_hp_test + 40,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        // -18%, mana max = 200
        assert_eq!(
            old_mana_launcher - (0.18 * old_mana_launcher as f64) as u64,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[MANA]
                .current
        );
        // "Magic power"
        // "ChangeMaxStatByPercentage" 15
        // +15%, mag power max = 20
        assert_eq!(
            old_mag_pow_test2 + (0.15 * old_mag_pow_test2 as f64) as u64,
            gm.pm
                .get_active_hero_character("test2_#1")
                .unwrap()
                .stats
                .all_stats[MAGICAL_POWER]
                .max
        );
        assert_eq!(
            old_mag_pow_test + (0.15 * old_mag_pow_test as f64) as u64,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[MAGICAL_POWER]
                .max
        );
        // "Physical power"
        // "ChangeMaxStatByPercentage" 15
        // +15%, phy power max = 10
        assert_eq!(
            old_phy_pow_test2 + (0.15 * old_phy_pow_test2 as f64).round() as u64,
            gm.pm
                .get_active_hero_character("test2_#1")
                .unwrap()
                .stats
                .all_stats[PHYSICAL_POWER]
                .max
        );
        assert_eq!(
            old_phy_pow_test + (0.15 * old_phy_pow_test as f64) as u64,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[PHYSICAL_POWER]
                .max
        );
    }

    #[test]
    fn unit_eclat_despoir_buffs_thrain_physical_and_magical_power() {
        use crate::testing::testing_all_characters::dxrpg_game_manager;

        let mut gm = dxrpg_game_manager();
        gm.start_game();

        // Advance using new_round() (no attacks fired) so Thraïn's stats stay at
        // their equipment-only baseline with no accumulated combat buffs.
        let mut max_setup = 30;
        while !gm.pm.current_player.id_name.contains("Elara") && max_setup > 0 {
            gm.new_round();
            max_setup -= 1;
        }
        if !gm.pm.current_player.id_name.contains("Elara") {
            return;
        }

        // Disable crit and ensure full mana for determinism.
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let mana_max = gm.pm.current_player.stats.all_stats[MANA].max;
        gm.pm.current_player.stats.all_stats[MANA].current = mana_max;

        let thrain_id = gm
            .pm
            .active_heroes
            .iter()
            .find(|h| h.id_name.contains("Thraïn"))
            .map(|h| h.id_name.clone())
            .expect("Thraïn must be among the lotr heroes");

        let old_phy_pow = gm
            .pm
            .get_active_hero_character(&thrain_id)
            .unwrap()
            .stats
            .all_stats[PHYSICAL_POWER]
            .max;
        let old_mag_pow = gm
            .pm
            .get_active_hero_character(&thrain_id)
            .unwrap()
            .stats
            .all_stats[MAGICAL_POWER]
            .max;

        // Thraïn's magical power comes from equipment (max_raw == 0 but equip buffers
        // give a non-zero effective max).  Both stats must be non-zero to make this
        // test meaningful.
        assert!(
            old_mag_pow > 0,
            "Thraïn must have non-zero magical power from equipment"
        );
        assert!(old_phy_pow > 0, "Thraïn must have non-zero physical power");

        gm.launch_attack(Some("Eclat d'espoir"));

        // Both stats must be boosted by +15 % (integer arithmetic matches the engine).
        assert_eq!(
            old_phy_pow + (0.15 * old_phy_pow as f64) as u64,
            gm.pm
                .get_active_hero_character(&thrain_id)
                .unwrap()
                .stats
                .all_stats[PHYSICAL_POWER]
                .max,
            "Eclat d'espoir should boost Thraïn physical power by 15 %"
        );
        assert_eq!(
            old_mag_pow + (0.15 * old_mag_pow as f64) as u64,
            gm.pm
                .get_active_hero_character(&thrain_id)
                .unwrap()
                .stats
                .all_stats[MAGICAL_POWER]
                .max,
            "Eclat d'espoir should boost Thraïn magical power by 15 %"
        );
    }

    #[test]
    fn unit_launch_attack_end_of_effect() {
        let (mut gm, hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();

        // New descending-speed order: test2_#1(312) > test_#1(212) > test_boss2_#1(15) > test_boss1_#1(11)
        // Only one supplementary attack per turn (test2_#1 at speed 312 qualifies; test_#1 does not).
        // testing_test_ally1_vs_test_boss1 advanced to round 2 (test_#1); test2_#1 already played round 1.
        assert_eq!(gm.game_state.order_to_play.len(), 5);
        assert_eq!(gm.pm.current_player.id_name, hero_launcher_id_name);
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        // apply effect Magic power - up by % for 2 turns (active turn1+turn2, ends on turn 3)
        // launch_attack calls eval_end_of_round internally, which advances one round
        gm.launch_attack(Some("Eclat d'espoir"));
        // eval_end_of_round advanced to round 3 (boss2 — higher speed than boss1)
        assert_eq!(gm.pm.current_player.id_name, "test_boss2_#1".to_owned());
        // round 4 (boss1)
        gm.new_round();
        assert_eq!(gm.pm.current_player.id_name, "test_boss1_#1".to_owned());
        // turn 1 round 5 (test2 supplementary — only one supplementary per turn)
        gm.new_round();
        assert_eq!(gm.pm.current_player.id_name, "test2_#1".to_owned());
        // turn 2 round 1 (test2 — highest speed, acts first)
        gm.start_new_turn();
        assert_eq!(gm.pm.current_player.id_name, "test2_#1".to_owned());
        // 2 effects received from eclat d espoir (counter turn 1/2, still active)
        assert_eq!(
            gm.pm.current_player.character_rounds_info.all_effects.len(),
            2
        );
        // turn 2 round 2 (test)
        gm.new_round();
        assert_eq!(gm.pm.current_player.id_name, "test_#1".to_owned());
        // turn 2 round 3 (boss2)
        gm.new_round();
        assert_eq!(gm.pm.current_player.id_name, "test_boss2_#1".to_owned());
        // turn 2 round 4 (boss1)
        gm.new_round();
        assert_eq!(gm.pm.current_player.id_name, "test_boss1_#1".to_owned());
        // turn 2 round 5 (test2 supplementary — only one supplementary per turn)
        gm.new_round();
        assert_eq!(gm.pm.current_player.id_name, "test2_#1".to_owned());
        // turn 3 round 1: test2_#1 was reset twice (312→212→112), so test_#1 (212) acts first now
        gm.start_new_turn();
        assert_eq!(gm.pm.current_player.id_name, "test_#1".to_owned());
        // effects ended after 2 turns
        assert!(
            gm.pm
                .current_player
                .character_rounds_info
                .all_effects
                .is_empty()
        );
    }

    #[test]
    fn unit_launch_attack_up_par_valeur() {
        let (mut gm, hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();

        assert_eq!(gm.pm.current_player.id_name, hero_launcher_id_name);
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_dodge = gm
            .pm
            .get_mut_active_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[DODGE]
            .max;
        let result = gm.launch_attack(Some("up-par-valeur"));
        let new_dodge = gm
            .pm
            .get_mut_active_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[DODGE]
            .max;
        assert_eq!(result.new_game_atk_effects.len(), 1);
        assert_eq!(new_dodge, old_dodge + 20);
    }

    #[test]
    fn unit_launch_attack_changement_par_value_berserk() {
        let (mut gm, hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();

        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_berserk_current = gm
            .pm
            .get_mut_active_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[BERSERK]
            .current;
        let old_berserk_max = gm
            .pm
            .get_mut_active_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[BERSERK]
            .max;
        let result = gm.launch_attack(Some("ChangeCurrentStatByValue-berseck"));
        let new_berserk = gm
            .pm
            .get_mut_active_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[BERSERK]
            .current;
        assert_eq!(result.new_game_atk_effects.len(), 1); // target himself
        // cost: -5% of berserk max, effect value +20
        assert_eq!(
            new_berserk,
            old_berserk_current - (5 * old_berserk_max / 100) + 20
        );
    }

    #[test]
    fn unit_launch_attack_case_cooldown() {
        let (mut gm, _hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();

        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let result = gm.launch_attack(Some("cooldown"));
        assert!(gm.game_state.status != GameStatus::EndOfGame);
        assert_eq!(result.new_game_atk_effects.len(), 1);
        assert_eq!(
            result
                .new_game_atk_effects
                .first()
                .unwrap()
                .processed_effect_param
                .input_effect_param
                .buffer
                .kind,
            BufKinds::CooldownTurnsNumber
        );
    }

    #[test]
    fn unit_integ_dxrpg() {
        let mut gm = testing_all_characters::dxrpg_game_manager();
        gm.start_game();
        let old_hp_boss = gm
            .pm
            .get_active_boss_character("Angmar_#1")
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        gm.pm
            .get_mut_active_boss_character("Angmar_#1")
            .unwrap()
            .character_rounds_info
            .is_current_target = true;
        // thrain
        // game is starting, ennemy is not playing
        assert_eq!(0, gm.process_nb_bosses_atk_in_a_row());
        let ra = gm.launch_attack(Some("SimpleAtk"));
        if !ra.all_dodging.is_empty() && ra.all_dodging[0].is_dodging {
            assert_eq!(
                old_hp_boss,
                gm.pm
                    .get_active_boss_character("Angmar_#1")
                    .unwrap()
                    .stats
                    .all_stats[HP]
                    .current
            );
        } else {
            let mut crit_coeff = 1;
            if ra.is_crit {
                crit_coeff = COEFF_CRIT_DMG as u64;
            }
            assert!(
                old_hp_boss - 31 * crit_coeff
                    >= gm
                        .pm
                        .get_active_boss_character("Angmar_#1")
                        .unwrap()
                        .stats
                        .all_stats[HP]
                        .current
            );
        }
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(2, gm.game_state.current_round);
        // elara
        assert_eq!(0, gm.process_nb_bosses_atk_in_a_row());
        let _ra = gm.launch_attack(Some("SimpleAtk"));
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(3, gm.game_state.current_round);
        let _ra = gm.launch_attack(Some("SimpleAtk"));
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(4, gm.game_state.current_round);
        let _ra = gm.launch_attack(Some("SimpleAtk"));
        assert!(gm.game_state.status != GameStatus::EndOfGame);
        assert_eq!(GameStatus::StartRound, gm.game_state.status);
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(5, gm.game_state.current_round);
        // check if a boss is auto playing
        assert!(gm.is_round_auto());
        let nb_bosses_atk = gm.process_nb_bosses_atk_in_a_row();
        assert!(nb_bosses_atk >= 1, "at least one boss should be attacking");
        // None => random atk for boss
        let _ = gm.launch_attack(None); // one or several hero could be dead
        let (all_heroes_dead, all_bosses_dead) = gm.pm.check_end_of_game();
        assert!(!all_heroes_dead);
        assert!(!all_bosses_dead);
        if !all_heroes_dead && !all_bosses_dead {
            assert_eq!(GameStatus::StartRound, gm.game_state.status);
            assert_eq!(1, gm.game_state.current_turn_nb);
            // round 6 is next boss round (still in boss sequence)
            let nb_remaining_bosses = gm.process_nb_bosses_atk_in_a_row();
            assert!(nb_remaining_bosses >= 0);
            // None => random atk for boss
            let _ = gm.launch_attack(None); // one or several hero could be dead
            let (all_heroes_dead, all_bosses_dead) = gm.pm.check_end_of_game();
            if !all_heroes_dead && !all_bosses_dead {
                assert_eq!(GameStatus::StartRound, gm.game_state.status);
                // With many bosses active, the turn count and round are variable
                let _ = gm.process_nb_bosses_atk_in_a_row();
            }
        }

        // ensure there is no dead lock -> game can be ended
        while gm.game_state.status == GameStatus::StartRound {
            let _ra = gm.launch_attack(Some("SimpleAtk"));
        }
        // On Linux and Windows the RNG differs, so the game may end because all heroes
        // die (EndOfGame) or because the last boss is killed first (EndOfScenario).
        // Both are valid terminal states; the important thing is that the loop exits.
        assert!(
            matches!(
                gm.game_state.status,
                GameStatus::EndOfGame | GameStatus::EndOfScenario
            ),
            "expected a terminal game state, got {:?}",
            gm.game_state.status
        );
    }

    #[test]
    fn unit_launch_attack_boss_pattern_queue() {
        let mut gm = testing_all_characters::testing_game_manager();

        // Set pattern [0, 2] for test_boss1_#1:
        // index 0 = first attack in boss's attacks_list
        // index 2 = third attack in boss's attacks_list
        gm.current_scenario
            .boss_patterns
            .insert("test_boss1".to_string(), vec![0, 2]);

        // start game and navigate to test_boss1_#1's round
        gm.start_game();
        while gm.pm.current_player.id_name != "test_boss1_#1" {
            let (ok, _) = gm.new_round();
            if !ok {
                gm.start_new_turn();
            }
        }

        // queue must be empty before first use
        assert!(
            gm.pm
                .current_player
                .character_rounds_info
                .atk_pattern_queue
                .is_empty(),
            "queue should be empty before first pattern launch"
        );

        // first launch: fills queue with [0, 2], pops 0, boss attacks using atk at index 0
        let ra1 = gm.launch_attack(None);
        assert_ne!(
            ra1.launcher_id_name, "",
            "expected a valid attack to be launched"
        );
        // queue now has [2] stored back in active_bosses
        let queue_after_first: Vec<u64> = gm
            .pm
            .get_active_boss_character("test_boss1_#1")
            .unwrap()
            .character_rounds_info
            .atk_pattern_queue
            .iter()
            .copied()
            .collect();
        assert_eq!(
            queue_after_first,
            vec![2u64],
            "queue should hold [2] after first launch"
        );

        // navigate back to test_boss1_#1's round
        while gm.pm.current_player.id_name != "test_boss1_#1" {
            let (ok, _) = gm.new_round();
            if !ok {
                gm.start_new_turn();
            }
        }

        // second launch: pops index 2, queue becomes empty
        let ra2 = gm.launch_attack(None);
        assert_ne!(ra2.launcher_id_name, "");
        let queue_after_second: Vec<u64> = gm
            .pm
            .get_active_boss_character("test_boss1_#1")
            .unwrap()
            .character_rounds_info
            .atk_pattern_queue
            .iter()
            .copied()
            .collect();
        assert!(
            queue_after_second.is_empty(),
            "queue should be empty after second launch"
        );

        // navigate back to test_boss1_#1's round
        while gm.pm.current_player.id_name != "test_boss1_#1" {
            let (ok, _) = gm.new_round();
            if !ok {
                gm.start_new_turn();
            }
        }

        // third launch: queue empty → refills [0, 2], pops 0 again (cycling)
        let ra3 = gm.launch_attack(None);
        assert_ne!(ra3.launcher_id_name, "");
        let queue_after_third: Vec<u64> = gm
            .pm
            .get_active_boss_character("test_boss1_#1")
            .unwrap()
            .character_rounds_info
            .atk_pattern_queue
            .iter()
            .copied()
            .collect();
        assert_eq!(
            queue_after_third,
            vec![2u64],
            "queue should hold [2] again after cycling"
        );
    }

    /// Pattern [0] must always use the attack at index 0 — never any other attack.
    /// This is the regression test for the bug where the pattern lookup used id_name
    /// ("test_boss1_#1") instead of db_full_name ("test_boss1"), causing the lookup
    /// to silently fail and fall through to random attack selection.
    #[test]
    fn unit_boss_pattern_single_index_always_same_atk() {
        let mut gm = testing_all_characters::testing_game_manager();

        // Pattern [0] keyed by db_full_name — only the first attack must ever be used.
        gm.current_scenario
            .boss_patterns
            .insert("test_boss1".to_string(), vec![0]);

        gm.start_game();

        let atk_at_index_0 = gm
            .pm
            .get_active_boss_character("test_boss1_#1")
            .unwrap()
            .attacks_list
            .get_index(0)
            .map(|(name, _)| name.clone())
            .expect("boss must have at least one attack");

        // Run 3 full boss turns and assert the same attack is used each time.
        for turn in 1..=3 {
            while gm.pm.current_player.id_name != "test_boss1_#1" {
                let (ok, _) = gm.new_round();
                if !ok {
                    gm.start_new_turn();
                }
            }
            let ra = gm.launch_attack(None);
            assert_eq!(
                ra.atk_name, atk_at_index_0,
                "turn {turn}: expected pattern attack '{}', got '{}'",
                atk_at_index_0, ra.atk_name
            );
        }
    }

    #[test]
    fn unit_load_next_scenario() {
        use crate::server::scenario::ScenarioState;

        let mut gm = testing_all_characters::dxrpg_game_manager();

        // dxrpg loads stage_1 and stage_2; states start as NotStarted
        let stage1_name = "Stage 1".to_owned();
        let stage2_name = "Stage 2".to_owned();
        assert_eq!(gm.states_scenarios[&stage1_name], ScenarioState::NotStarted);
        assert_eq!(gm.states_scenarios[&stage2_name], ScenarioState::NotStarted);

        // set stage 1 as current (simulates game start on stage 1)
        let stage1 = gm
            .all_scenarios
            .iter()
            .find(|s| s.name == stage1_name)
            .cloned()
            .unwrap();
        gm.current_scenario = stage1;
        gm.states_scenarios
            .insert(stage1_name.clone(), ScenarioState::InProgress);

        // damage heroes and drain their energy to verify restoration on next scenario
        for hero in gm.pm.active_heroes.iter_mut() {
            hero.stats.get_mut_value(HP).current = 1;
            hero.stats.get_mut_value(MANA).current = 0;
            hero.stats.get_mut_value(VIGOR).current = 0;
            hero.stats.get_mut_value(BERSERK).current = 0;
        }

        // load stage 2
        let result = gm.load_next_scenario();
        assert!(result.is_ok(), "loading stage 2 should succeed");

        // stage 1 must be Completed
        assert_eq!(
            gm.states_scenarios[&stage1_name],
            ScenarioState::Completed,
            "stage 1 should be Completed after loading stage 2"
        );
        // stage 2 must be InProgress
        assert_eq!(
            gm.states_scenarios[&stage2_name],
            ScenarioState::InProgress,
            "stage 2 should be InProgress after being loaded"
        );
        // current_scenario must be stage 2
        assert_eq!(gm.current_scenario.name, stage2_name);

        // active_bosses count must equal the stage 2 boss patterns
        assert_eq!(
            gm.pm.active_bosses.len(),
            gm.current_scenario.boss_patterns.len(),
            "active_bosses should match stage 2 boss patterns count"
        );

        // heroes must have HP, energy and no effects restored to max
        for hero in gm.pm.active_heroes.iter() {
            assert_eq!(
                hero.stats.all_stats[HP].current, hero.stats.all_stats[HP].max,
                "hero {} HP should be restored to max",
                hero.db_full_name
            );
            assert_eq!(
                hero.stats.all_stats[MANA].current, hero.stats.all_stats[MANA].max,
                "hero {} Mana should be restored to max",
                hero.db_full_name
            );
            assert_eq!(
                hero.stats.all_stats[VIGOR].current, hero.stats.all_stats[VIGOR].max,
                "hero {} Vigor should be restored to max",
                hero.db_full_name
            );
            assert_eq!(
                hero.stats.all_stats[BERSERK].current, 0,
                "hero {} Berserk should NOT be restored on scenario load",
                hero.db_full_name
            );
            assert!(
                hero.character_rounds_info.all_effects.is_empty(),
                "hero {} should have no active effects after scenario transition",
                hero.db_full_name
            );
        }

        // all_scenarios_completed returns false (stage 2 still in progress)
        assert!(!gm.all_scenarios_completed());
    }

    #[test]
    fn unit_set_active_bosses() {
        use crate::testing::testing_all_characters::dxrpg_dm;

        let dm = dxrpg_dm();
        let mut gm = testing_all_characters::dxrpg_game_manager();

        // set stage 1 as current scenario so boss_patterns are in scope
        let stage1 = gm
            .all_scenarios
            .iter()
            .find(|s| s.level == 1)
            .cloned()
            .unwrap();
        gm.current_scenario = stage1;

        // no bosses yet
        gm.pm.active_bosses.clear();
        assert_eq!(gm.pm.active_bosses.len(), 0);

        gm.set_active_bosses(&dm.all_bosses);

        // the number of active bosses must match the number of boss_patterns entries
        // that have a matching entry in dm.all_bosses
        let expected = gm
            .current_scenario
            .boss_patterns
            .keys()
            .filter(|name| dm.all_bosses.iter().any(|b| &b.db_full_name == *name))
            .count();
        assert_eq!(
            gm.pm.active_bosses.len(),
            expected,
            "active_bosses count should match boss_patterns with a known boss"
        );

        // each active boss must have the correct id_name suffix format
        for boss in &gm.pm.active_bosses {
            assert!(
                boss.id_name.contains("_#"),
                "id_name '{}' should contain '_#'",
                boss.id_name
            );
        }
    }

    // -------------------------------------------------------------------------
    // process_end_of_scenario tests
    // -------------------------------------------------------------------------

    #[test]
    fn unit_end_of_scenario_equipment_loot_matching_class() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        // Both test heroes are Standard class.
        // Create a scenario with one equipment loot targeting Standard heroes.
        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "starting right weapon".to_string(),
                kind: LootType::Equipment,
                rank: Rank::Common,
                level: 1,
                classes: vec![Class::Standard],
            }],
            universe: String::new(),
        };

        gm.process_end_of_scenario();

        // Both heroes must now have the equipment in their inventory
        for hero in &gm.pm.active_heroes {
            let has_it = hero
                .inventory
                .equipments
                .values()
                .flatten()
                .any(|e| e.unique_name == "starting right weapon");
            assert!(
                has_it,
                "hero '{}' should have received the equipment",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_equipment_loot_no_class_match() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        // Both test heroes are Standard.
        // Equipment loot only for Warrior → heroes must NOT receive an extra copy.
        let belts_before: Vec<usize> = gm
            .pm
            .active_heroes
            .iter()
            .map(|h| {
                h.inventory
                    .equipments
                    .values()
                    .flatten()
                    .filter(|e| e.unique_name == "starting belt")
                    .count()
            })
            .collect();

        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "starting belt".to_string(),
                kind: LootType::Equipment,
                rank: Rank::Common,
                level: 1,
                classes: vec![Class::Warrior],
            }],
            universe: String::new(),
        };

        gm.process_end_of_scenario();

        for (idx, hero) in gm.pm.active_heroes.iter().enumerate() {
            let belts_after = hero
                .inventory
                .equipments
                .values()
                .flatten()
                .filter(|e| e.unique_name == "starting belt")
                .count();
            assert_eq!(
                belts_after, belts_before[idx],
                "hero '{}' belt count should not change (class mismatch)",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_equipment_loot_unknown_equipment() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        // Record initial equipment count per hero
        let equip_before: Vec<usize> = gm
            .pm
            .active_heroes
            .iter()
            .map(|h| h.inventory.equipments.values().map(|v| v.len()).sum())
            .collect();

        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "non_existent_equipment".to_string(),
                kind: LootType::Equipment,
                rank: Rank::Common,
                level: 1,
                classes: vec![Class::Standard],
            }],
            universe: String::new(),
        };

        // Must not panic; unknown equipment is just warned about and skipped
        gm.process_end_of_scenario();

        for (idx, hero) in gm.pm.active_heroes.iter().enumerate() {
            let total_equip: usize = hero.inventory.equipments.values().map(|v| v.len()).sum();
            assert_eq!(
                total_equip, equip_before[idx],
                "hero '{}' equipment count should not change for unknown loot name",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_consumable_loot() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();

        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "Common potion".to_string(),
                kind: LootType::Consumable,
                rank: Rank::Common, // heals 20 HP
                level: 1,
                classes: vec![Class::Standard],
            }],
            universe: String::new(),
        };

        gm.process_end_of_scenario();

        // Consumables must go to the shared party bag — not to individual heroes.
        let in_party_bag = gm
            .pm
            .party_consumables
            .iter()
            .any(|c| c.name == "Common potion");
        assert!(
            in_party_bag,
            "consumable should land in the party bag, not in individual inventories"
        );

        // Personal inventories must be untouched.
        for hero in &gm.pm.active_heroes {
            let in_personal_bag = hero
                .inventory
                .consumables
                .iter()
                .any(|c| c.name == "Common potion");
            assert!(
                !in_personal_bag,
                "hero '{}' should NOT have the consumable in their personal bag",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_build_consumable_effects_named_potions() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();

        for (potion_name, rank) in [
            ("potion of resurrection", Rank::Advanced),
            ("mana potion", Rank::Intermediate),
            ("vigor potion", Rank::Common),
            ("berserk potion", Rank::Advanced),
        ] {
            let original_len = gm.pm.party_consumables.len();
            gm.current_scenario = Scenario {
                name: "test".to_string(),
                description: "test".to_string(),
                boss_patterns: HashMap::new(),
                level: 1,
                loots: vec![Loot {
                    name: potion_name.to_string(),
                    kind: LootType::Consumable,
                    rank,
                    level: 1,
                    classes: vec![Class::Standard],
                }],
                universe: String::new(),
            };
            gm.process_end_of_scenario();
            let found = gm
                .pm
                .party_consumables
                .iter()
                .skip(original_len)
                .any(|c| c.name == potion_name);
            assert!(
                found,
                "'{potion_name}' should be in party bag after end_of_scenario"
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_currency_loot() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "gold".to_string(),
                kind: LootType::Currency,
                rank: Rank::Common,
                level: 100,
                classes: vec![Class::Standard],
            }],
            universe: String::new(),
        };

        // Test heroes already have money: 100 in their JSON
        let money_before: Vec<u64> = gm
            .pm
            .active_heroes
            .iter()
            .map(|h| h.inventory.money)
            .collect();

        gm.process_end_of_scenario();

        for (idx, hero) in gm.pm.active_heroes.iter().enumerate() {
            assert_eq!(
                hero.inventory.money,
                money_before[idx] + 100,
                "hero '{}' should have received 100 gold",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_exp_and_level_up() {
        // Test setup: 2 bosses, each rank Common level 1 → 100 exp each → 200 total
        //
        // "test" hero:  exp=50, exp_to_next_level(Common, Standard, 1)=100
        //   50 + 200 = 250 → level-up to 2 (exp=150), new threshold=200 → 150 < 200 → stop at level 2
        //
        // "test2" hero: exp=0,  exp_to_next_level(Common, Standard, 1)=100
        //   0 + 200 = 200 → level-up to 2 (exp=100), new threshold=200 → 100 < 200 → stop at level 2
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            loots: vec![],
            level: 1,
            universe: String::new(),
        };

        let old_hp_max: Vec<u64> = gm
            .pm
            .active_heroes
            .iter()
            .map(|h| h.stats.all_stats[HP].max)
            .collect();

        gm.process_end_of_scenario();

        for (idx, hero) in gm.pm.active_heroes.iter().enumerate() {
            assert_eq!(
                hero.level, 2,
                "hero '{}' should be level 2 after 200 exp (dynamic threshold grows to 200 at level 2)",
                hero.id_name
            );
            // exp_to_next_level must now reflect the new level
            assert_eq!(
                hero.character_rounds_info.exp_to_next_level,
                200, // build_exp_to_next_level(Common, Standard, 2) = 200
                "hero '{}' exp_to_next_level should be 200 at level 2",
                hero.id_name
            );
            // Stats must have been updated upward on level-up
            assert!(
                hero.stats.all_stats[HP].max > old_hp_max[idx],
                "hero '{}' HP max should have increased after leveling up",
                hero.id_name
            );
        }
        // assess end of scenario LevelUp
        assert_eq!(gm.end_of_scenario.characters_levelup.len(), 2); // 2 heroes
        gm.end_of_scenario.characters_levelup.iter().for_each(|lu| {
            assert_eq!(
                lu.new_level, 2,
                "LevelUp record should show new level 2 for hero '{}'",
                lu.character_id_name
            );
            assert_eq!(
                lu.old_level, 1,
                "LevelUp record should show old level 1 for hero '{}'",
                lu.character_id_name
            );
        });
    }

    #[test]
    fn unit_end_of_scenario_no_level_up_when_exp_insufficient() {
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        // Remove all bosses so total_exp = 0
        gm.pm.active_bosses.clear();
        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            loots: vec![],
            level: 1,
            universe: String::new(),
        };

        let levels_before: Vec<u64> = gm.pm.active_heroes.iter().map(|h| h.level).collect();

        gm.process_end_of_scenario();

        for (idx, hero) in gm.pm.active_heroes.iter().enumerate() {
            assert_eq!(
                hero.level, levels_before[idx],
                "hero '{}' should not have leveled up with 0 exp",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_triggered_via_game_flow() {
        // Verify that eval_end_of_round sets EndOfScenario and processes rewards
        // when all bosses are killed in a single hit.
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let (mut gm, _, _) = testing_test_ally1_vs_test_boss1();

        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "gold".to_string(),
                kind: LootType::Currency,
                rank: Rank::Common,
                level: 50,
                classes: vec![Class::Standard],
            }],
            universe: String::new(),
        };

        // Kill all bosses
        for boss in gm.pm.active_bosses.iter_mut() {
            boss.stats.all_stats.get_mut(HP).unwrap().current = 0;
        }

        // Set target and launch — eval_end_of_round sees all bosses dead
        gm.pm
            .get_mut_active_boss_character("test_boss1_#1")
            .unwrap()
            .character_rounds_info
            .is_current_target = true;
        gm.launch_attack(None);

        assert_eq!(
            gm.game_state.status,
            GameStatus::EndOfScenario,
            "status should be EndOfScenario"
        );
        // Rewards must have been processed: each Standard hero got 50 gold on top of their starting 100
        for hero in &gm.pm.active_heroes {
            assert!(
                hero.inventory.money >= 50,
                "hero '{}' should have received 50 gold after end-of-scenario",
                hero.id_name
            );
        }
    }

    /// Fracas Marteau deals self-damage via its buffer effects on the caster.
    /// This test verifies that a low-HP hero is killed by the self-damage component.
    #[test]
    fn unit_fracas_marteau_can_kill_caster() {
        use crate::{
            character_mod::{attack_type::AttackType, buffers::BufKinds, effect::EffectParam},
            common::constants::{
                all_target_const::TARGET_HIMSELF, reach_const::INDIVIDUAL, stats_const::HP,
            },
        };

        let (mut gm, hero_id_name, _) = testing_test_ally1_vs_test_boss1();

        // Build a self-damage attack: 50 HP self-damage (guaranteed kill at 10 HP)
        use crate::character_mod::buffers::Buffer;
        let fracas_marteau = AttackType {
            name: "Fracas Marteau".to_owned(),
            target: TARGET_HIMSELF.to_owned(),
            reach: INDIVIDUAL.to_owned(),
            all_effects: vec![EffectParam {
                nb_turns: 1,
                target_kind: TARGET_HIMSELF.to_owned(),
                reach: INDIVIDUAL.to_owned(),
                buffer: Buffer {
                    kind: BufKinds::ChangeCurrentStatByValue,
                    value: -50,
                    is_percent: false,
                    stats_name: HP.to_owned(),
                    is_passive_enabled: false,
                    is_passive: false,
                },
                ..Default::default()
            }],
            ..Default::default()
        };

        // Set hero HP to 10 so self-damage is lethal
        for hero in gm.pm.active_heroes.iter_mut() {
            if hero.id_name == hero_id_name {
                hero.stats.get_mut_value(HP).current = 10;
                hero.attacks_list
                    .insert(fracas_marteau.name.clone(), fracas_marteau.clone());
            }
        }
        // Also update current_player (shadow copy)
        if gm.pm.current_player.id_name == hero_id_name {
            gm.pm.current_player.stats.get_mut_value(HP).current = 10;
            gm.pm
                .current_player
                .attacks_list
                .insert(fracas_marteau.name.clone(), fracas_marteau.clone());
        }

        let result = gm.launch_attack(Some(&fracas_marteau.name));

        // The attack must have been launched by our hero
        assert_eq!(
            result.launcher_id_name, hero_id_name,
            "Fracas Marteau should be launched by {hero_id_name}"
        );
        // There must be at least one HP effect on the caster
        assert!(
            !result.new_game_atk_effects.is_empty(),
            "Fracas Marteau should produce at least one game effect"
        );

        // The hero should be dead after taking 50+ self-damage from 10 HP
        let hero_after = gm
            .pm
            .active_heroes
            .iter()
            .find(|h| h.id_name == hero_id_name);
        if let Some(hero) = hero_after {
            assert!(
                hero.stats.is_dead() == Some(true) || hero.stats.all_stats[HP].current == 0,
                "Fracas Marteau should kill the hero at 10 HP, but HP is {}",
                hero.stats.all_stats[HP].current
            );
        }
    }

    // ── Aggro integration tests ────────────────────────────────────────────────

    /// After a damage attack the launcher's aggro should be strictly greater than its initial value.
    #[test]
    fn unit_aggro_increases_after_damage_attack() {
        use crate::common::constants::stats_const::AGGRO;

        let (mut gm, hero_launcher_id_name, target_id_name) = testing_test_ally1_vs_test_boss1();

        // Disable dodge & crit so the attack lands cleanly.
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .stats
            .all_stats[DODGE]
            .current = 0;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;

        let aggro_before = gm.pm.current_player.stats.all_stats[AGGRO].current;

        let ra = gm.launch_attack(Some("SimpleAtk"));

        // Verify the attack actually dealt damage.
        assert!(
            !ra.new_game_atk_effects.is_empty(),
            "SimpleAtk should produce at least one effect"
        );

        // After a damage attack the aggro should have grown.
        // Re-read current_player stats from the updated copy stored in active_heroes.
        let aggro_after = gm
            .pm
            .active_heroes
            .iter()
            .find(|h| h.id_name == hero_launcher_id_name)
            .map(|h| h.stats.all_stats[AGGRO].current)
            .unwrap_or(0);

        assert!(
            aggro_after > aggro_before,
            "Aggro should increase after a damage attack: before={aggro_before}, after={aggro_after}"
        );
    }

    /// After a damage attack the launcher's DamageTx for the current turn should reflect
    /// the magnitude of the damage dealt.
    #[test]
    fn unit_damage_tx_filled_after_damage_attack() {
        use crate::character_mod::rounds_information::AmountType;

        let (mut gm, hero_launcher_id_name, target_id_name) = testing_test_ally1_vs_test_boss1();

        // Disable dodge & crit so the attack lands cleanly.
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .stats
            .all_stats[DODGE]
            .current = 0;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;

        let turn_nb = gm.game_state.current_turn_nb as u64;

        let ra = gm.launch_attack(Some("SimpleAtk"));
        assert!(
            !ra.new_game_atk_effects.is_empty(),
            "SimpleAtk should produce at least one effect"
        );

        // The turn advances after the attack, so read the launcher from active_heroes
        // (current_player now points to the next acting character).
        let damage_tx = gm
            .pm
            .active_heroes
            .iter()
            .find(|h| h.id_name == hero_launcher_id_name)
            .and_then(|h| {
                h.character_rounds_info
                    .tx_rx
                    .get(AmountType::DamageTx as usize)
                    .and_then(|m| m.get(&turn_nb))
                    .copied()
            })
            .unwrap_or(0);

        assert!(
            damage_tx > 0,
            "DamageTx should be filled with the damage dealt this turn, got {damage_tx}"
        );
    }

    /// Aggro from two consecutive attacks accumulates (not reset to base each time).
    #[test]
    fn unit_aggro_accumulates_across_attacks() {
        use crate::common::constants::stats_const::AGGRO;

        let (mut gm, hero_launcher_id_name, target_id_name) = testing_test_ally1_vs_test_boss1();

        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .stats
            .all_stats[DODGE]
            .current = 0;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;

        // First attack.
        let _ra1 = gm.launch_attack(Some("SimpleAtk"));

        // Sync current_player with the updated hero stats so second attack uses same launcher.
        if let Some(updated_hero) = gm
            .pm
            .active_heroes
            .iter()
            .find(|h| h.id_name == hero_launcher_id_name)
        {
            gm.pm.current_player = updated_hero.clone();
        }

        // Re-target boss for second attack.
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;

        let aggro_after_first = gm
            .pm
            .active_heroes
            .iter()
            .find(|h| h.id_name == hero_launcher_id_name)
            .map(|h| h.stats.all_stats[AGGRO].current)
            .unwrap_or(0);

        // Second attack.
        let _ra2 = gm.launch_attack(Some("SimpleAtk"));

        let aggro_after_second = gm
            .pm
            .active_heroes
            .iter()
            .find(|h| h.id_name == hero_launcher_id_name)
            .map(|h| h.stats.all_stats[AGGRO].current)
            .unwrap_or(0);

        assert!(
            aggro_after_second >= aggro_after_first,
            "Aggro must not decrease between consecutive attacks: first={aggro_after_first}, second={aggro_after_second}"
        );
    }

    /// Aggro accumulates correctly across full turn cycles (hero→boss→hero full loop).
    /// This verifies the real game flow where eval_end_of_round advances all other characters.
    #[test]
    fn unit_aggro_accumulates_across_full_turns() {
        use crate::common::constants::stats_const::AGGRO;
        use crate::server::game_state::GameStatus;

        let (mut gm, hero_launcher_id_name, target_id_name) = testing_test_ally1_vs_test_boss1();

        // Disable dodge and critical strike variance for determinism.
        gm.pm.current_player.stats.all_stats[DODGE].current = 0;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        if let Some(boss) = gm.pm.get_mut_active_boss_character(&target_id_name) {
            boss.character_rounds_info.is_current_target = true;
        }
        for h in gm.pm.active_heroes.iter_mut() {
            h.stats.all_stats[DODGE].current = 0;
            h.stats.all_stats[CRITICAL_STRIKE].current = 0;
        }

        // --- Turn 1: hero attacks ---
        let _ra1 = gm.launch_attack(Some("SimpleAtk"));
        let aggro_after_turn1 = gm
            .pm
            .active_heroes
            .iter()
            .find(|h| h.id_name == hero_launcher_id_name)
            .map(|h| h.stats.all_stats[AGGRO].current)
            .unwrap_or(0);

        // Advance through remaining rounds of turn 1 (all non-hero players auto-attack),
        // then through all of turn 2 until it is hero's turn again.
        let mut max_rounds = 50; // safety cap to avoid infinite loop
        while gm.pm.current_player.id_name != hero_launcher_id_name
            && gm.game_state.status != GameStatus::EndOfGame
            && gm.game_state.status != GameStatus::EndOfScenario
            && max_rounds > 0
        {
            let _ = gm.launch_attack(None);
            max_rounds -= 1;
        }

        // Abort if the game ended early (e.g. hero died to auto-attacks).
        if gm.game_state.status == GameStatus::EndOfGame
            || gm.game_state.status == GameStatus::EndOfScenario
        {
            return;
        }

        // Re-enable target so second hero attack hits.
        if let Some(boss) = gm.pm.get_mut_active_boss_character(&target_id_name) {
            boss.character_rounds_info.is_current_target = true;
        }

        // --- Turn 2: hero attacks again ---
        let _ra2 = gm.launch_attack(Some("SimpleAtk"));
        let aggro_after_turn2 = gm
            .pm
            .active_heroes
            .iter()
            .find(|h| h.id_name == hero_launcher_id_name)
            .map(|h| h.stats.all_stats[AGGRO].current)
            .unwrap_or(0);

        assert!(
            aggro_after_turn2 >= aggro_after_turn1,
            "Aggro should not decrease between turn 1 and turn 2: turn1={aggro_after_turn1}, turn2={aggro_after_turn2}"
        );
        assert!(
            aggro_after_turn2 > 0,
            "Aggro should be positive after two attacks: turn2={aggro_after_turn2}"
        );
    }

    /// Aggro accumulates correctly for a real LOTR hero (Thraïn) using "Frappe Cinglante"
    /// across two consecutive turns.  Uses dxrpg_game_manager() so actual hero data is tested.
    #[test]
    fn unit_aggro_thrain_frappe_cinglante_accumulates() {
        use crate::common::constants::stats_const::AGGRO;
        use crate::server::game_state::GameStatus;
        use crate::testing::testing_all_characters::dxrpg_game_manager;

        let mut gm = dxrpg_game_manager();
        gm.start_game();

        // Advance until Thraïn is the current player.
        let mut max_setup = 30;
        while !gm.pm.current_player.id_name.contains("Thraïn")
            && gm.game_state.status != GameStatus::EndOfGame
            && gm.game_state.status != GameStatus::EndOfScenario
            && max_setup > 0
        {
            gm.launch_attack(None);
            max_setup -= 1;
        }
        // Skip if Thraïn isn't up, or if the scenario already ended on Linux (bosses
        // died before Thraïn's first turn — no valid targets remain for the assertion).
        if !gm.pm.current_player.id_name.contains("Thraïn")
            || gm.game_state.status != GameStatus::StartRound
        {
            return;
        }

        let thrain_id = gm.pm.current_player.id_name.clone();
        // Disable dodge & critical variance for determinism
        gm.pm.current_player.stats.all_stats[DODGE].current = 0;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        if let Some(boss) = gm
            .pm
            .active_bosses
            .iter_mut()
            .find(|b| !b.stats.is_dead().unwrap_or(false))
        {
            boss.character_rounds_info.is_current_target = true;
        }

        // Turn 1: Thraïn attacks with "Frappe Cinglante " (trailing space matches filename)
        let ra1 = gm.launch_attack(Some("Frappe Cinglante "));
        let aggro_t1 = gm
            .pm
            .active_heroes
            .iter()
            .find(|h| h.id_name == thrain_id)
            .map(|h| h.stats.all_stats[AGGRO].current)
            .unwrap_or(0);
        assert!(
            !ra1.new_game_atk_effects.is_empty(),
            "Frappe Cinglante should produce at least one effect"
        );

        // Advance through all rounds until Thraïn can attack again (next turn).
        let mut max_rounds = 60;
        while gm.pm.current_player.id_name != thrain_id
            && gm.game_state.status != GameStatus::EndOfGame
            && gm.game_state.status != GameStatus::EndOfScenario
            && max_rounds > 0
        {
            gm.launch_attack(None);
            max_rounds -= 1;
        }
        if gm.game_state.status == GameStatus::EndOfGame
            || gm.game_state.status == GameStatus::EndOfScenario
            || gm.pm.current_player.id_name != thrain_id
        {
            return; // game ended, skip
        }

        gm.pm.current_player.stats.all_stats[DODGE].current = 0;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        if let Some(boss) = gm
            .pm
            .active_bosses
            .iter_mut()
            .find(|b| !b.stats.is_dead().unwrap_or(false))
        {
            boss.character_rounds_info.is_current_target = true;
        }

        // Turn 2: Thraïn attacks again
        let _ra2 = gm.launch_attack(Some("Frappe Cinglante "));
        let aggro_t2 = gm
            .pm
            .active_heroes
            .iter()
            .find(|h| h.id_name == thrain_id)
            .map(|h| h.stats.all_stats[AGGRO].current)
            .unwrap_or(0);

        assert!(
            aggro_t2 >= aggro_t1,
            "Thraïn aggro must not decrease between turns: t1={aggro_t1}, t2={aggro_t2}"
        );
        assert!(
            aggro_t2 > 0,
            "Thraïn aggro must be > 0 after two attacks: {aggro_t2}"
        );
    }

    // ── Rameau Guérisseur tests ────────────────────────────────────────────────
    // The attack applies:
    //   1. DecreasingRateOnTurn HP HOT on an individual ally (nb_turns=4, value=3)
    //   2. ChangeMaxStatByPercentage +10% Magic power on the same target (nb_turns=4)
    //
    // Test char stats: Magical Power max=30 (20 raw + 10 from starting_gloves equipment),
    //   HP max=135, Mana 200/200.
    // HOT per-tick = applies * (buffer.value + magic_power_current / nb_turns)
    //              = applies * (3 + 30/4) = applies * 10  (integer division)
    // applies comes from process_decrease_on_turn(value=3): always at least 1
    // (first threshold is 100%), at most 3.  Per-tick ∈ [10, 30].
    //
    // The HOT then fires probabilistically on subsequent turns:
    //   T2 counter=1: threshold = (3−1+1)/3 = 100 % (always)
    //   T3 counter=2: threshold = (3−2+1)/3 =  67 %
    //   T4 counter=3: threshold = (3−3+1)/3 =  33 %
    // So the HOT fires 1–3 ticks after launch (not always 3).
    //
    // The DecreasingRateOnTurn effect also stores its applies count in ApplyEffectInit,
    // which is then picked up by ALL subsequent effects in the same attack.  So
    // the ChangeMaxStatByPercentage full_amount = applies * 10, giving a magic
    // power increase of 30 * (applies*10) / 100 = applies * 3.
    // New magic power max ∈ [33, 36, 39] for applies ∈ [1, 2, 3].

    #[test]
    fn unit_rameau_guerisseur_initial_heal_range() {
        let (mut gm, hero_id, _) = testing_test_ally1_vs_test_boss1();
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        if let Some(buf) = gm
            .pm
            .current_player
            .character_rounds_info
            .get_mut_buffer_by_type(&BufKinds::NextHealAtkIsCrit)
        {
            buf.is_passive_enabled = false;
        }
        gm.pm.set_targeted_characters(&hero_id, "Rameau Guérisseur");
        let old_hp = gm
            .pm
            .get_active_hero_character("test2_#1")
            .unwrap()
            .stats
            .all_stats[HP]
            .current;

        let ra = gm.launch_attack(Some("Rameau Guérisseur"));

        // Two effects: HP HOT + Magic power buff
        assert_eq!(ra.new_game_atk_effects.len(), 2, "expected 2 effects");

        let hot = ra
            .new_game_atk_effects
            .iter()
            .find(|g| {
                g.processed_effect_param
                    .input_effect_param
                    .buffer
                    .stats_name
                    == HP
            })
            .expect("HP effect missing");
        let per_tick = hot.effect_outcome.full_amount_tx;

        // applies ∈ [1, 3], per apply = 10  → per_tick ∈ [10, 30]
        assert!(
            per_tick >= 10,
            "per-tick heal below minimum (1 apply × 10): {per_tick}"
        );
        assert!(
            per_tick <= 30,
            "per-tick heal above maximum (3 applies × 10): {per_tick}"
        );

        // HP was immediately increased on launch
        let new_hp = gm
            .pm
            .get_active_hero_character("test2_#1")
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let hp_max = gm
            .pm
            .get_active_hero_character("test2_#1")
            .unwrap()
            .stats
            .all_stats[HP]
            .max;
        assert_eq!(
            new_hp,
            (old_hp as i64 + per_tick).clamp(0, hp_max as i64) as u64
        );
    }

    #[test]
    fn unit_rameau_guerisseur_magic_power_buff() {
        // The ChangeMaxStatByPercentage effect shares the ApplyEffectInit count
        // set by DecreasingRateOnTurn, so full_amount = applies * 10.
        // With old_magic_max = 30 (20 raw + 10 equipment):
        //   increase = 30 * (applies * 10) / 100 = applies * 3  → new ∈ [33, 39]
        let (mut gm, hero_id, _) = testing_test_ally1_vs_test_boss1();
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        if let Some(buf) = gm
            .pm
            .current_player
            .character_rounds_info
            .get_mut_buffer_by_type(&BufKinds::NextHealAtkIsCrit)
        {
            buf.is_passive_enabled = false;
        }

        let old_magic_max = gm
            .pm
            .get_active_hero_character("test2_#1")
            .unwrap()
            .stats
            .all_stats[MAGICAL_POWER]
            .max;

        gm.pm.set_targeted_characters(&hero_id, "Rameau Guérisseur");
        let ra = gm.launch_attack(Some("Rameau Guérisseur"));

        // Derive number_of_applies from the HOT's per-tick amount:
        // per_tick = applies * 10  →  applies = per_tick / 10
        let per_tick = ra
            .new_game_atk_effects
            .iter()
            .find(|g| {
                g.processed_effect_param
                    .input_effect_param
                    .buffer
                    .stats_name
                    == HP
            })
            .unwrap()
            .effect_outcome
            .full_amount_tx;
        let applies = per_tick / 10;

        let new_magic_max = gm
            .pm
            .get_active_hero_character("test2_#1")
            .unwrap()
            .stats
            .all_stats[MAGICAL_POWER]
            .max;

        // full_amount for magic buf = applies * 10 → increase = old * full_amount / 100
        let full_amount = applies * 10;
        let expected = old_magic_max + old_magic_max * full_amount as u64 / 100;
        assert_eq!(
            new_magic_max,
            expected,
            "Magic power should increase by {}% (applies={applies}): {old_magic_max} → {expected}, got {new_magic_max}",
            applies * 10
        );
        // Sanity: increase is proportional to applies (1-3)
        assert!(
            new_magic_max >= old_magic_max + old_magic_max * 10 / 100,
            "min expected +10% increase: got {new_magic_max}"
        );
        assert!(
            new_magic_max <= old_magic_max + old_magic_max * 30 / 100,
            "max expected +30% increase: got {new_magic_max}"
        );
    }

    #[test]
    fn unit_rameau_guerisseur_hot_lasts_exactly_4_turns() {
        // The effect entry persists for exactly nb_turns=4 turns regardless of how many
        // ticks actually fired (which is probabilistic: 1–3). Expiry is always at T5.
        let (mut gm, hero_id, _) = testing_test_ally1_vs_test_boss1();
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        if let Some(buf) = gm
            .pm
            .current_player
            .character_rounds_info
            .get_mut_buffer_by_type(&BufKinds::NextHealAtkIsCrit)
        {
            buf.is_passive_enabled = false;
        }
        gm.pm.set_targeted_characters(&hero_id, "Rameau Guérisseur");
        gm.launch_attack(Some("Rameau Guérisseur"));

        // Advance to test2's first round in T1 — HOT skipped (same launch turn)
        while gm.pm.current_player.id_name != "test2_#1" {
            gm.new_round();
        }
        assert_eq!(
            gm.pm.current_player.character_rounds_info.all_effects.len(),
            2,
            "both effects must be present in T1"
        );

        // T2, T3, T4: effects still active when test2 plays each turn
        for turn_idx in 2..=4 {
            gm.start_new_turn();
            while gm.pm.current_player.id_name != "test2_#1" {
                gm.new_round();
            }
            assert_eq!(
                gm.pm.current_player.character_rounds_info.all_effects.len(),
                2,
                "both effects must still be active at turn {turn_idx}"
            );
        }

        // T5: counter reaches nb_turns=4 → both effects removed before HOT fires
        gm.start_new_turn();
        while gm.pm.current_player.id_name != "test2_#1" {
            gm.new_round();
        }
        assert!(
            gm.pm
                .current_player
                .character_rounds_info
                .all_effects
                .is_empty(),
            "effects must expire after exactly 4 turns (nb_turns=4)"
        );
    }

    #[test]
    fn unit_rameau_guerisseur_hot_fires_at_most_3_ticks() {
        // Verifies the HOT fires AT MOST 3 times (T2, T3, T4) but not necessarily
        // exactly 3: the DecreasingRateOnTurn probability means T2=100%, T3=67%,
        // T4=33%. So the HOT fires 1–3 times depending on the random rolls.
        let (mut gm, hero_id, _) = testing_test_ally1_vs_test_boss1();
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        if let Some(buf) = gm
            .pm
            .current_player
            .character_rounds_info
            .get_mut_buffer_by_type(&BufKinds::NextHealAtkIsCrit)
        {
            buf.is_passive_enabled = false;
        }
        gm.pm.set_targeted_characters(&hero_id, "Rameau Guérisseur");
        gm.launch_attack(Some("Rameau Guérisseur"));

        // Skip T1 (HOT does not fire same turn as launch)
        while gm.pm.current_player.id_name != "test2_#1" {
            gm.new_round();
        }

        // HP regen per turn for test2 (7); HOT tick ≥10 — any increase > regen means HOT fired
        let regen = gm
            .pm
            .get_active_hero_character("test2_#1")
            .unwrap()
            .stats
            .all_stats[HP_REGEN]
            .current as i64;

        let mut hot_ticks = 0u32;
        for _ in 2..=4 {
            // Capture HP before start_new_turn because test2_#1 is first in new order:
            // start_new_turn processes round=1 (test2_#1) which applies HOT immediately.
            let hp_before = gm
                .pm
                .get_active_hero_character("test2_#1")
                .unwrap()
                .stats
                .all_stats[HP]
                .current as i64;
            gm.start_new_turn();
            // After start_new_turn, test2_#1 is current (round=1) with HOT+regen applied.
            let hp_after = gm
                .pm
                .get_active_hero_character("test2_#1")
                .unwrap()
                .stats
                .all_stats[HP]
                .current as i64;
            // HOT (≥10) + regen (7) >> regen alone (7)
            if hp_after - hp_before > regen {
                hot_ticks += 1;
            }
        }

        assert!(
            hot_ticks >= 1,
            "HOT must fire at least once (T2 is always 100%): fired {hot_ticks} times"
        );
        assert!(
            hot_ticks <= 3,
            "HOT must fire at most 3 times (T2–T4): fired {hot_ticks} times"
        );
    }

    #[test]
    fn unit_new_round_all_heroes_dead_end_of_game() {
        let mut gm = testing_game_manager();
        gm.start_game();
        // Kill ALL heroes
        for hero in &mut gm.pm.active_heroes {
            hero.stats.all_stats[HP].current = 0;
        }
        // Make round 1 point to the first hero (who is dead)
        gm.game_state.current_round = 0;
        let (is_new_round, _logs) = gm.new_round();
        assert!(
            !is_new_round,
            "dead player → should not start a new round normally"
        );
        assert_eq!(
            gm.game_state.status,
            GameStatus::EndOfGame,
            "all heroes dead → EndOfGame"
        );
    }

    /// Offrande vitale must boost Thraïn's magical and physical armor max by +50%.
    #[test]
    fn unit_offrande_vitale_buffs_thrain_armor() {
        use crate::testing::testing_all_characters::dxrpg_game_manager;

        let mut gm = dxrpg_game_manager();
        gm.start_game();

        // Advance to Elara's turn
        let mut max_setup = 30;
        while !gm.pm.current_player.id_name.contains("Elara") && max_setup > 0 {
            gm.new_round();
            max_setup -= 1;
        }
        if !gm.pm.current_player.id_name.contains("Elara") {
            return;
        }

        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let mana_max = gm.pm.current_player.stats.all_stats[MANA].max;
        gm.pm.current_player.stats.all_stats[MANA].current = mana_max;

        let thrain_id = gm
            .pm
            .active_heroes
            .iter()
            .find(|h| h.id_name.contains("Thraïn"))
            .map(|h| h.id_name.clone())
            .expect("Thraïn must be present");

        let old_mag_armor = gm
            .pm
            .get_active_hero_character(&thrain_id)
            .unwrap()
            .stats
            .all_stats[MAGICAL_ARMOR]
            .max;
        let old_phy_armor = gm
            .pm
            .get_active_hero_character(&thrain_id)
            .unwrap()
            .stats
            .all_stats[PHYSICAL_ARMOR]
            .max;

        // Offrande vitale targets a single ally — manually mark Thraïn as the current target.
        gm.pm
            .get_mut_active_hero_character(&thrain_id)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;

        gm.launch_attack(Some("Offrande vitale"));

        let new_mag_armor = gm
            .pm
            .get_active_hero_character(&thrain_id)
            .unwrap()
            .stats
            .all_stats[MAGICAL_ARMOR]
            .max;
        let new_phy_armor = gm
            .pm
            .get_active_hero_character(&thrain_id)
            .unwrap()
            .stats
            .all_stats[PHYSICAL_ARMOR]
            .max;

        assert_eq!(
            old_mag_armor + old_mag_armor * 50 / 100,
            new_mag_armor,
            "Offrande vitale must boost Thraïn magic armor max by 50%"
        );
        assert_eq!(
            old_phy_armor + old_phy_armor * 50 / 100,
            new_phy_armor,
            "Offrande vitale must boost Thraïn physical armor max by 50%"
        );
    }

    /// Bouclier Défensif must give exactly +40 aggro to Thraïn (not +42 from implicit Berserk aggro).
    #[test]
    fn unit_bouclier_defensif_exact_aggro() {
        use crate::testing::testing_all_characters::dxrpg_game_manager;

        let mut gm = dxrpg_game_manager();
        gm.start_game();

        // Advance to Thraïn's turn
        let mut max_setup = 30;
        while !gm.pm.current_player.id_name.contains("Thraïn") && max_setup > 0 {
            gm.new_round();
            max_setup -= 1;
        }
        if !gm.pm.current_player.id_name.contains("Thraïn") {
            return;
        }

        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;

        let thrain_id = gm.pm.current_player.id_name.clone();
        let aggro_before = gm.pm.current_player.stats.all_stats[AGGRO].current;

        // Bouclier Défensif targets Self — no explicit target setting needed.
        gm.launch_attack(Some("Bouclier Défensif "));

        let aggro_after = gm
            .pm
            .get_active_hero_character(&thrain_id)
            .unwrap()
            .stats
            .all_stats[AGGRO]
            .current;

        assert_eq!(
            aggro_before + 40,
            aggro_after,
            "Bouclier Défensif must give exactly +40 aggro (not inflated by Berserk implicit aggro)"
        );
    }

    /// Fureur Déchaînée targets Self: no enemy is harmed, Thraïn's Physical power
    /// max increases by 30 %, and his aggro increases by the explicit +5 aggro effect.
    #[test]
    fn unit_fureur_dechainee_self_only() {
        use crate::testing::testing_all_characters::dxrpg_game_manager;

        let mut gm = dxrpg_game_manager();
        gm.start_game();

        // Advance to Thraïn's turn (hard limit to avoid an infinite loop).
        let mut max_rounds = 30;
        while !gm.pm.current_player.id_name.contains("Thraïn") && max_rounds > 0 {
            gm.new_round();
            max_rounds -= 1;
        }
        if !gm.pm.current_player.id_name.contains("Thraïn") {
            return;
        }

        // No crit so the result is deterministic.
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        // Ensure enough Berserk for the attack (cost = 12).
        gm.pm.current_player.stats.all_stats[BERSERK].current = 50;

        let thrain_id = gm.pm.current_player.id_name.clone();

        let old_phy_pow_max = gm
            .pm
            .get_active_hero_character(&thrain_id)
            .unwrap()
            .stats
            .all_stats[PHYSICAL_POWER]
            .max;
        let old_aggro = gm
            .pm
            .get_active_hero_character(&thrain_id)
            .unwrap()
            .stats
            .all_stats[AGGRO]
            .current;

        // Record every boss HP before the attack.
        let boss_hp_before: Vec<(String, u64)> = gm
            .pm
            .active_bosses
            .iter()
            .map(|b| (b.id_name.clone(), b.stats.all_stats[HP].current))
            .collect();

        // Launch Fureur Déchaînée (target: Self — attack name has two trailing spaces).
        let result = gm.launch_attack(Some("Fureur Déchaînée  "));

        // --- No effect must land on any enemy ---
        for gae in &result.new_game_atk_effects {
            let target = &gae.effect_outcome.target_id_name;
            assert!(
                gm.pm.get_active_boss_character(target).is_none(),
                "Fureur Déchaînée must not affect any boss; got effect on '{target}'"
            );
        }

        // Boss HP must be unchanged.
        for (boss_id, hp_before) in &boss_hp_before {
            let hp_after = gm
                .pm
                .get_active_boss_character(boss_id)
                .map(|b| b.stats.all_stats[HP].current)
                .unwrap_or(*hp_before);
            assert_eq!(
                *hp_before, hp_after,
                "Boss '{boss_id}' HP must be unchanged after Fureur Déchaînée"
            );
        }

        // --- Self-buff: Physical power max must be +30 % ---
        let new_phy_pow_max = gm
            .pm
            .get_active_hero_character(&thrain_id)
            .unwrap()
            .stats
            .all_stats[PHYSICAL_POWER]
            .max;
        assert_eq!(
            old_phy_pow_max + old_phy_pow_max * 30 / 100,
            new_phy_pow_max,
            "Fureur Déchaînée must boost Thraïn's Physical power max by 30 %"
        );

        // --- Explicit aggro effect: +5 aggro on Thraïn ---
        let new_aggro = gm
            .pm
            .get_active_hero_character(&thrain_id)
            .unwrap()
            .stats
            .all_stats[AGGRO]
            .current;
        assert_eq!(
            old_aggro + 5,
            new_aggro,
            "Fureur Déchaînée must give Thraïn exactly +5 aggro"
        );
    }

    // -------------------------------------------------------------------------
    // Eveil de la forêt (Thalia) — integration tests
    // -------------------------------------------------------------------------

    fn setup_thalia_turn() -> (super::GameManager, String) {
        use crate::testing::testing_all_characters::dxrpg_game_manager;

        let mut gm = dxrpg_game_manager();
        gm.start_game();

        let mut max_rounds = 30;
        while !gm.pm.current_player.id_name.contains("Thalia") && max_rounds > 0 {
            gm.new_round();
            max_rounds -= 1;
        }
        // If Thalia never became current player the test is a no-op (guard at call site).
        let thalia_id = gm.pm.current_player.id_name.clone();
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let mana_max = gm.pm.current_player.stats.all_stats[MANA].max;
        gm.pm.current_player.stats.all_stats[MANA].current = mana_max;
        (gm, thalia_id)
    }

    /// Eveil de la forêt boosts Magic power max by +10% on every ally.
    #[test]
    fn unit_eveil_foret_boosts_magic_power_all_allies() {
        let (mut gm, thalia_id) = setup_thalia_turn();
        if !thalia_id.contains("Thalia") {
            return;
        }

        let old_mag_pow: Vec<(String, u64)> = gm
            .pm
            .active_heroes
            .iter()
            .filter(|h| h.id_name != thalia_id)
            .map(|h| (h.id_name.clone(), h.stats.all_stats[MAGICAL_POWER].max))
            .collect();
        let old_thalia_mag_pow = gm.pm.current_player.stats.all_stats[MAGICAL_POWER].max;

        gm.launch_attack(Some("Eveil de la forêt"));

        for (id, old_val) in &old_mag_pow {
            let new_val =
                gm.pm.get_active_hero_character(id).unwrap().stats.all_stats[MAGICAL_POWER].max;
            assert_eq!(
                old_val + old_val * 10 / 100,
                new_val,
                "Eveil de la forêt must boost {id} Magic power max by 10%"
            );
        }
        // Also applies to the caster herself (All allies target includes self)
        let new_thalia_mag_pow = gm
            .pm
            .get_active_hero_character(&thalia_id)
            .unwrap()
            .stats
            .all_stats[MAGICAL_POWER]
            .max;
        assert_eq!(
            old_thalia_mag_pow + old_thalia_mag_pow * 10 / 100,
            new_thalia_mag_pow,
            "Eveil de la forêt must boost Thalia's own Magic power max by 10%"
        );
    }

    /// Eveil de la forêt applies a +80 HP HOT (4 turns) to every ally except the caster.
    /// The "Ally" + Zone target kind intentionally excludes the launcher.
    #[test]
    fn unit_eveil_foret_hot_on_all_allies() {
        use crate::character_mod::effect::is_hot;

        let (mut gm, thalia_id) = setup_thalia_turn();
        if !thalia_id.contains("Thalia") {
            return;
        }

        gm.launch_attack(Some("Eveil de la forêt"));

        for hero in gm
            .pm
            .active_heroes
            .iter()
            .filter(|h| h.id_name != thalia_id)
        {
            let hp_hot_count = hero
                .character_rounds_info
                .all_effects
                .iter()
                .filter(|gae| {
                    is_hot(
                        &gae.processed_effect_param.input_effect_param.buffer.kind,
                        &gae.processed_effect_param
                            .input_effect_param
                            .buffer
                            .stats_name,
                        gae.processed_effect_param.input_effect_param.buffer.value,
                    )
                })
                .count();
            assert!(
                hp_hot_count >= 1,
                "Eveil de la forêt must apply at least one HP HOT on {}",
                hero.id_name
            );

            let hot_effect = hero
                .character_rounds_info
                .all_effects
                .iter()
                .find(|gae| {
                    is_hot(
                        &gae.processed_effect_param.input_effect_param.buffer.kind,
                        &gae.processed_effect_param
                            .input_effect_param
                            .buffer
                            .stats_name,
                        gae.processed_effect_param.input_effect_param.buffer.value,
                    )
                })
                .unwrap();
            assert_eq!(
                hot_effect
                    .processed_effect_param
                    .input_effect_param
                    .nb_turns,
                4,
                "HOT from Eveil de la forêt must last 4 turns on {}",
                hero.id_name
            );
        }
    }

    /// Eveil de la forêt removes one debuff from every ally except the caster.
    /// RemoveOneDebuf uses "Ally" + Zone which intentionally excludes the launcher.
    #[test]
    fn unit_eveil_foret_removes_one_debuff_from_all_allies() {
        use crate::character_mod::buffers::BufKinds;
        use crate::character_mod::effect::is_debuf_effect;

        let (mut gm, thalia_id) = setup_thalia_turn();
        if !thalia_id.contains("Thalia") {
            return;
        }

        // Inject a DOT debuff on each ally except Thalia (she is excluded by "Ally" zone).
        for hero in gm.pm.active_heroes.iter_mut() {
            if hero.id_name.contains("Thalia") {
                continue;
            }
            let dot = crate::character_mod::effect::ProcessedEffectParam {
                input_effect_param: crate::character_mod::effect::EffectParam {
                    nb_turns: 3,
                    buffer: crate::character_mod::buffers::Buffer {
                        kind: BufKinds::ChangeCurrentStatByValue,
                        value: -20,
                        is_percent: false,
                        stats_name: HP.to_owned(),
                        is_passive_enabled: false,
                        is_passive: false,
                    },
                    ..Default::default()
                },
                number_of_applies: 1,
                ..Default::default()
            };
            let gae = crate::server::players_manager::GameAtkEffect {
                processed_effect_param: dot,
                atk_type: Default::default(),
                launching_turn: 1,
                launching_round: 1,
                effect_outcome: Default::default(),
            };
            hero.character_rounds_info.all_effects.push(gae);
        }

        // Verify each ally now has a debuff.
        for hero in gm
            .pm
            .active_heroes
            .iter()
            .filter(|h| !h.id_name.contains("Thalia"))
        {
            assert!(
                hero.character_rounds_info
                    .all_effects
                    .iter()
                    .any(|gae| is_debuf_effect(&gae.processed_effect_param.input_effect_param)),
                "Setup: {} must have a debuff before launch",
                hero.id_name
            );
        }

        let debuff_counts_before: Vec<(String, usize)> = gm
            .pm
            .active_heroes
            .iter()
            .filter(|h| !h.id_name.contains("Thalia"))
            .map(|h| {
                let count = h
                    .character_rounds_info
                    .all_effects
                    .iter()
                    .filter(|gae| is_debuf_effect(&gae.processed_effect_param.input_effect_param))
                    .count();
                (h.id_name.clone(), count)
            })
            .collect();

        gm.launch_attack(Some("Eveil de la forêt"));

        for (id, count_before) in &debuff_counts_before {
            let count_after = gm
                .pm
                .get_active_hero_character(id)
                .unwrap()
                .character_rounds_info
                .all_effects
                .iter()
                .filter(|gae| is_debuf_effect(&gae.processed_effect_param.input_effect_param))
                .count();
            assert_eq!(
                count_before - 1,
                count_after,
                "Eveil de la forêt must remove exactly one debuff from {id}"
            );
        }
    }

    /// Eveil de la forêt puts a 10-turn cooldown on Thalia.
    #[test]
    fn unit_eveil_foret_sets_cooldown_on_thalia() {
        use crate::character_mod::buffers::BufKinds;

        let (mut gm, thalia_id) = setup_thalia_turn();
        if !thalia_id.contains("Thalia") {
            return;
        }

        gm.launch_attack(Some("Eveil de la forêt"));

        let thalia = gm.pm.get_active_hero_character(&thalia_id).unwrap();
        let cooldown_effect = thalia.character_rounds_info.all_effects.iter().find(|gae| {
            gae.processed_effect_param.input_effect_param.buffer.kind
                == BufKinds::CooldownTurnsNumber
        });
        assert!(
            cooldown_effect.is_some(),
            "Eveil de la forêt must apply a CooldownTurnsNumber effect on Thalia"
        );
        assert_eq!(
            cooldown_effect
                .unwrap()
                .processed_effect_param
                .input_effect_param
                .nb_turns,
            10,
            "Eveil de la forêt cooldown must last 10 turns"
        );
    }

    /// Eveil de la forêt boosts all active HOTs by +33% via BoostHotsByPercentage.
    /// Only allies other than the caster are checked because the HOT is "Ally" zone
    /// (excludes self); "All allies" zone effects do include the caster.
    #[test]
    fn unit_eveil_foret_boosts_hots_by_33_percent() {
        use crate::character_mod::buffers::BufKinds;
        use crate::character_mod::effect::is_hot;

        let (mut gm, thalia_id) = setup_thalia_turn();
        if !thalia_id.contains("Thalia") {
            return;
        }

        // Pre-seed each ally (including current_player so modify_active_character
        // does not overwrite Thalia's entry) with a HOT so there is something to boost.
        let pre_hot_ep = crate::character_mod::effect::ProcessedEffectParam {
            input_effect_param: crate::character_mod::effect::EffectParam {
                nb_turns: 4,
                buffer: crate::character_mod::buffers::Buffer {
                    kind: BufKinds::ChangeCurrentStatByValue,
                    value: 60,
                    is_percent: false,
                    stats_name: HP.to_owned(),
                    is_passive_enabled: false,
                    is_passive: false,
                },
                ..Default::default()
            },
            number_of_applies: 1,
            ..Default::default()
        };
        let pre_hot_gae = crate::server::players_manager::GameAtkEffect {
            processed_effect_param: pre_hot_ep,
            atk_type: Default::default(),
            launching_turn: 1,
            launching_round: 1,
            effect_outcome: Default::default(),
        };
        for hero in gm.pm.active_heroes.iter_mut() {
            hero.character_rounds_info
                .all_effects
                .push(pre_hot_gae.clone());
        }
        gm.pm
            .current_player
            .character_rounds_info
            .all_effects
            .push(pre_hot_gae);

        gm.launch_attack(Some("Eveil de la forêt"));

        // After launch, every non-caster hero's pre-seeded HOT (60) should be boosted to 79
        // (60 + 33% of 60 = 79 floor). The BoostHotsByPercentage effect has "All allies"
        // zone, so it fires for Thalia too — but we only check non-caster heroes here since
        // the Eveil HOT (value=80) lands only on non-caster allies.
        for hero in gm
            .pm
            .active_heroes
            .iter()
            .filter(|h| h.id_name != thalia_id)
        {
            let boosted_hot = hero
                .character_rounds_info
                .all_effects
                .iter()
                .filter(|gae| {
                    is_hot(
                        &gae.processed_effect_param.input_effect_param.buffer.kind,
                        &gae.processed_effect_param
                            .input_effect_param
                            .buffer
                            .stats_name,
                        gae.processed_effect_param.input_effect_param.buffer.value,
                    ) && gae.processed_effect_param.input_effect_param.buffer.value >= 79
                        && gae.processed_effect_param.input_effect_param.buffer.value < 100
                })
                .count();
            assert!(
                boosted_hot >= 1,
                "Eveil de la forêt must have boosted the pre-seeded HOT (60→79) on {}",
                hero.id_name
            );
        }
    }

    /// Eveil de la forêt reinitialises existing HP HOT counters (ReinitBuf effect).
    /// ReinitBuf uses "All allies" zone so it applies to every ally including the caster.
    #[test]
    fn unit_eveil_foret_reinit_hot_counters() {
        use crate::character_mod::buffers::BufKinds;

        let (mut gm, thalia_id) = setup_thalia_turn();
        if !thalia_id.contains("Thalia") {
            return;
        }

        // Seed a HOT that is partially consumed (counter_turn = 2).
        // Must be added to both active_heroes AND current_player so that
        // modify_active_character (which copies current_player → active_heroes[thalia])
        // does not overwrite Thalia's entry and erase the seed.
        let aged_hot_ep = crate::character_mod::effect::ProcessedEffectParam {
            input_effect_param: crate::character_mod::effect::EffectParam {
                nb_turns: 4,
                buffer: crate::character_mod::buffers::Buffer {
                    kind: BufKinds::ChangeCurrentStatByValue,
                    value: 50,
                    is_percent: false,
                    stats_name: HP.to_owned(),
                    is_passive_enabled: false,
                    is_passive: false,
                },
                ..Default::default()
            },
            counter_turn: 2,
            number_of_applies: 1,
            ..Default::default()
        };
        let aged_hot_gae = crate::server::players_manager::GameAtkEffect {
            processed_effect_param: aged_hot_ep,
            atk_type: Default::default(),
            launching_turn: 1,
            launching_round: 1,
            effect_outcome: Default::default(),
        };
        for hero in gm.pm.active_heroes.iter_mut() {
            hero.character_rounds_info
                .all_effects
                .push(aged_hot_gae.clone());
        }
        gm.pm
            .current_player
            .character_rounds_info
            .all_effects
            .push(aged_hot_gae);

        gm.launch_attack(Some("Eveil de la forêt"));

        // ReinitBuf resets counter_turn to 0 on every HP HOT for all allies.
        for hero in &gm.pm.active_heroes {
            let reset = hero.character_rounds_info.all_effects.iter().any(|gae| {
                gae.processed_effect_param.input_effect_param.buffer.kind
                    == BufKinds::ChangeCurrentStatByValue
                    && gae
                        .processed_effect_param
                        .input_effect_param
                        .buffer
                        .stats_name
                        == HP
                    && gae.processed_effect_param.input_effect_param.buffer.value > 0
                    && gae.processed_effect_param.counter_turn == 0
            });
            assert!(
                reset,
                "Eveil de la forêt ReinitBuf must reset counter_turn=0 on HP HOT for {}",
                hero.id_name
            );
        }
    }

    /// Eveil de la forêt sets a BoostedByHots buffer on Thalia proportional to her active HOT count.
    #[test]
    fn unit_eveil_foret_boosted_by_hots_on_thalia() {
        use crate::character_mod::buffers::BufKinds;
        use crate::character_mod::effect::is_hot;

        let (mut gm, thalia_id) = setup_thalia_turn();
        if !thalia_id.contains("Thalia") {
            return;
        }

        // Count HOTs currently active on Thalia before launch.
        let hot_count_before = gm
            .pm
            .current_player
            .character_rounds_info
            .all_effects
            .iter()
            .filter(|gae| {
                is_hot(
                    &gae.processed_effect_param.input_effect_param.buffer.kind,
                    &gae.processed_effect_param
                        .input_effect_param
                        .buffer
                        .stats_name,
                    gae.processed_effect_param.input_effect_param.buffer.value,
                )
            })
            .count() as i64;

        gm.launch_attack(Some("Eveil de la forêt"));

        // BoostBufByHotsNumberInPercentage fires BEFORE the zone HOT is added to Thalia,
        // so BoostedByHots value = hot_count_before * 20.
        let thalia = gm.pm.get_active_hero_character(&thalia_id).unwrap();
        let boosted = thalia
            .character_rounds_info
            .get_buffer_by_type(&BufKinds::BoostedByHots);
        assert!(
            boosted.is_some(),
            "Eveil de la forêt must set a BoostedByHots buffer on Thalia"
        );
        assert_eq!(
            hot_count_before * 20,
            boosted.unwrap().value,
            "BoostedByHots value must equal number_of_hots × 20"
        );
    }

    /// Eveil de la forêt (BoostHotsByPercentage +33%) must boost a pre-existing HOT on
    /// Azrak Ombresang — simulating the HOT he would have received from Essence Régénératrice.
    /// This covers the zone-target bug where only the caster's HOTs were previously boosted.
    #[test]
    fn unit_eveil_foret_boosts_azrak_existing_hot() {
        use crate::character_mod::buffers::BufKinds;
        use crate::character_mod::effect::is_hot;

        let (mut gm, thalia_id) = setup_thalia_turn();
        if !thalia_id.contains("Thalia") {
            return;
        }

        let azrak_id = gm
            .pm
            .active_heroes
            .iter()
            .find(|h| h.id_name.contains("Azrak"))
            .map(|h| h.id_name.clone())
            .expect("Azrak Ombresang must be in the lotr party");

        // Seed a +12 HP HOT on Azrak (as Essence Régénératrice would give).
        let hot_value: i64 = 12;
        let hot_ep = crate::character_mod::effect::ProcessedEffectParam {
            input_effect_param: crate::character_mod::effect::EffectParam {
                nb_turns: 4,
                buffer: crate::character_mod::buffers::Buffer {
                    kind: BufKinds::ChangeCurrentStatByValue,
                    value: hot_value,
                    is_percent: false,
                    stats_name: HP.to_owned(),
                    is_passive_enabled: false,
                    is_passive: false,
                },
                ..Default::default()
            },
            number_of_applies: 1,
            ..Default::default()
        };
        let hot_gae = crate::server::players_manager::GameAtkEffect {
            processed_effect_param: hot_ep,
            atk_type: Default::default(),
            launching_turn: 1,
            launching_round: 1,
            effect_outcome: crate::character_mod::effect::EffectOutcome {
                full_amount_tx: hot_value,
                real_amount_tx: hot_value,
                target_id_name: azrak_id.clone(),
                ..Default::default()
            },
        };
        gm.pm
            .get_mut_active_hero_character(&azrak_id)
            .unwrap()
            .character_rounds_info
            .all_effects
            .push(hot_gae);

        gm.launch_attack(Some("Eveil de la forêt"));

        // After Eveil de la forêt, Azrak's HOT value must be boosted by +33%.
        let azrak = gm.pm.get_active_hero_character(&azrak_id).unwrap();
        let azrak_hot = azrak
            .character_rounds_info
            .all_effects
            .iter()
            .find(|gae| {
                is_hot(
                    &gae.processed_effect_param.input_effect_param.buffer.kind,
                    &gae.processed_effect_param
                        .input_effect_param
                        .buffer
                        .stats_name,
                    gae.processed_effect_param.input_effect_param.buffer.value,
                ) && gae.processed_effect_param.input_effect_param.buffer.value
                    >= hot_value + hot_value * 33 / 100
                    && gae.processed_effect_param.input_effect_param.buffer.value
                        <= hot_value + hot_value * 33 / 100 + 1
            })
            .expect("Azrak's HOT must be boosted by +33% by Eveil de la forêt");

        let boosted = hot_value + hot_value * 33 / 100;
        assert_eq!(
            boosted,
            azrak_hot
                .processed_effect_param
                .input_effect_param
                .buffer
                .value,
            "Azrak's HOT buffer.value must be boosted from {hot_value} to {boosted} (+33%)"
        );
        assert_eq!(
            boosted, azrak_hot.effect_outcome.full_amount_tx,
            "Azrak's HOT effect_outcome.full_amount_tx must be boosted so ticks heal {boosted} HP"
        );
        assert_eq!(
            boosted, azrak_hot.effect_outcome.real_amount_tx,
            "Azrak's HOT effect_outcome.real_amount_tx must be boosted so log_text() shows {boosted} HP"
        );
    }

    /// Integration test: 3 heroes (Elara, Azrak, Thalia) + 1 enemy.
    /// Elara's IsDamageTxHealNeedyAlly passive fires immediately when she deals damage,
    /// healing the most-needy alive ally and emitting a log in the same turn.
    #[test]
    fn unit_passive_damage_tx_heal_needy_3_heroes_1_enemy() {
        let mut gm = testing_all_characters::dxrpg_game_manager();
        gm.pm.active_heroes.retain(|h| {
            matches!(
                h.id_name.as_str(),
                "Elara_la_guerisseuse_de_la_Lorien_#1" | "Azrak_Ombresang_#1" | "Thalia_#1"
            )
        });
        gm.pm.active_bosses.truncate(1);

        let elara_id = "Elara_la_guerisseuse_de_la_Lorien_#1";
        let azrak_id = "Azrak_Ombresang_#1";

        // Drain Azrak to 10 HP — lowest ratio → most needy
        gm.pm
            .get_mut_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .all_stats
            .get_mut(HP)
            .unwrap()
            .current = 10;
        let azrak_hp_max = gm
            .pm
            .get_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .max;

        // Set Elara as current player, no crit
        let elara = gm.pm.get_active_hero_character(elara_id).unwrap().clone();
        gm.pm.current_player = elara;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;

        // Boss: no dodge, no armor bonus, no DamageRxPercent
        let boss_id = gm.pm.active_bosses[0].id_name.clone();
        gm.pm.active_bosses[0].stats.all_stats[DODGE].current = 0;
        gm.pm.active_bosses[0].stats.all_stats[MAGICAL_ARMOR].current = 0;
        gm.pm.active_bosses[0]
            .character_rounds_info
            .is_current_target = true;
        if let Some(buf) = gm.pm.active_bosses[0]
            .character_rounds_info
            .get_mut_buffer_by_type(&BufKinds::DamageRxPercent)
        {
            buf.value = 0;
        }

        let old_boss_hp = gm.pm.active_bosses[0].stats.all_stats[HP].current;

        let result = gm.launch_attack(Some("SimpleAtk"));

        // Derive actual damage from boss HP delta (avoids hardcoding hero magic power)
        let new_boss_hp = gm
            .pm
            .get_active_boss_character(&boss_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let actual_damage = old_boss_hp as i64 - new_boss_hp as i64;
        assert!(actual_damage > 0, "SimpleAtk must deal damage to the boss");

        // Passive fires in the same turn: 25% of damage heals Azrak (most needy)
        let expected_heal = ((actual_damage as u64 * 25 / 100) as u64).min(azrak_hp_max - 10);
        let new_azrak_hp = gm
            .pm
            .get_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        assert_eq!(
            new_azrak_hp,
            10 + expected_heal,
            "Elara's passive must heal Azrak by 25% of damage dealt in the same turn"
        );

        // Passive log must appear in logs_atk of this attack result
        assert!(
            result
                .logs_atk
                .iter()
                .any(|l| l.message.contains("Passive")),
            "passive heal log must appear in logs_atk"
        );
    }

    /// Integration test: Thraïn's passive ChangeCurrentStatByPercentage(Dodge, 10) raises his
    /// effective Dodge from 5 to 15 at load time and survives an equipment toggle.
    #[test]
    fn unit_passive_dodge_stat_thrain_3_heroes_1_enemy() {
        use crate::testing::testing_all_characters::testing_all_equipment;

        let mut gm = testing_all_characters::dxrpg_game_manager();
        gm.pm.active_heroes.retain(|h| {
            matches!(
                h.id_name.as_str(),
                "Thraïn_#1" | "Azrak_Ombresang_#1" | "Thalia_#1"
            )
        });
        gm.pm.active_bosses.truncate(1);

        let thrain_id = "Thraïn_#1";

        // At load: base_value = max_raw(5) + equip(24: amulet+4, cape+10, shoes+10) = 29
        // passive: +10% of 29 = 2 (integer) → total = 31
        let dodge_after_load = gm
            .pm
            .get_active_hero_character(thrain_id)
            .unwrap()
            .stats
            .all_stats[DODGE]
            .current;
        assert_eq!(31, dodge_after_load, "passive must be included at load");

        // After removing the starting amulet (Dodge +4), base_value drops to 25.
        // passive: +10% of 25 = 2 (integer) → total = 27
        let thrain = gm.pm.get_mut_active_hero_character(thrain_id).unwrap();
        thrain.toggle_equipment("starting amulet", &testing_all_equipment());
        let dodge_after_toggle = thrain.stats.all_stats[DODGE].current;
        assert_eq!(
            27, dodge_after_toggle,
            "Dodge must be 27 after removing amulet (passive still applies)"
        );
    }

    #[test]
    fn unit_elara_frappe_elementaire_3_heroes_3_enemies() {
        use crate::character_mod::rounds_information::AmountType;

        let mut gm = testing_all_characters::dxrpg_game_manager();
        gm.pm.active_heroes.retain(|h| {
            matches!(
                h.id_name.as_str(),
                "Elara_la_guerisseuse_de_la_Lorien_#1" | "Azrak_Ombresang_#1" | "Thalia_#1"
            )
        });
        gm.pm.active_bosses.truncate(3);

        let elara_id = "Elara_la_guerisseuse_de_la_Lorien_#1";
        let elara = gm.pm.get_active_hero_character(elara_id).unwrap().clone();
        gm.pm.current_player = elara;
        gm.pm.current_player.level = 100;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;

        // Set target: first boss, no dodge
        let boss_id = gm.pm.active_bosses[0].id_name.clone();
        gm.pm.active_bosses[0].stats.all_stats[DODGE].current = 0;
        gm.pm.active_bosses[0]
            .character_rounds_info
            .is_current_target = true;
        // Angmar carries a DamageRxPercent:100 passive that would double damage;
        // zero it out so this test verifies the base armor-mitigation formula only.
        if let Some(buf) = gm.pm.active_bosses[0]
            .character_rounds_info
            .get_mut_buffer_by_type(&BufKinds::DamageRxPercent)
        {
            buf.value = 0;
        }

        let old_boss_hp = gm.pm.active_bosses[0].stats.all_stats[HP].current;
        let mana_max = gm.pm.current_player.stats.all_stats[MANA].max;
        let old_mana = gm.pm.current_player.stats.all_stats[MANA].current;

        // Expected magic damage: value=70, Elara magic power, boss magic armor
        let hero_mag_pow = gm.pm.current_player.stats.get_power_stat(true);
        let boss_mag_armor = gm.pm.active_bosses[0].stats.get_armor_stat(true);
        let raw_dmg = (70_i64 + hero_mag_pow) as f64;
        let protection =
            AttackType::ARMOR_FACTOR / (AttackType::ARMOR_FACTOR + boss_mag_armor as f64);
        let expected_dmg = (raw_dmg * protection).round() as i64;

        // RepeatIfHeal does not fire: no heal on turn 0
        gm.pm.current_player.character_rounds_info.tx_rx[AmountType::HealTx as usize].clear();

        let ra = gm.launch_attack(Some("Frappe élémentaire"));
        assert!(
            !ra.new_game_atk_effects.is_empty(),
            "attack must produce effects"
        );

        // Mana cost: 15% of mana_max
        assert_eq!(
            old_mana - 15 * mana_max / 100,
            gm.pm
                .get_active_hero_character(elara_id)
                .unwrap()
                .stats
                .all_stats[MANA]
                .current,
            "Elara mana cost: 15% of max"
        );

        // Boss HP reduced by expected magic damage
        let new_boss_hp = gm
            .pm
            .get_active_boss_character(&boss_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        assert_eq!(
            old_boss_hp as i64 - expected_dmg,
            new_boss_hp as i64,
            "boss HP reduced by magic damage"
        );
    }

    #[test]
    fn unit_elara_don_de_vie_3_heroes_3_enemies() {
        let mut gm = testing_all_characters::dxrpg_game_manager();
        gm.pm.active_heroes.retain(|h| {
            matches!(
                h.id_name.as_str(),
                "Elara_la_guerisseuse_de_la_Lorien_#1" | "Azrak_Ombresang_#1" | "Thalia_#1"
            )
        });
        gm.pm.active_bosses.truncate(3);

        let elara_id = "Elara_la_guerisseuse_de_la_Lorien_#1";
        let azrak_id = "Azrak_Ombresang_#1";

        let elara = gm.pm.get_active_hero_character(elara_id).unwrap().clone();
        gm.pm.current_player = elara;
        gm.pm.current_player.level = 100;

        // Drain Azrak to 50 HP and mark as ally target
        gm.pm
            .get_mut_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .current = 50;
        gm.pm
            .get_mut_active_hero_character(azrak_id)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;

        let azrak_hp_max = gm
            .pm
            .get_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .max;
        let old_elara_hp = gm.pm.current_player.stats.all_stats[HP].current;
        let elara_hp_max = gm.pm.current_player.stats.all_stats[HP].max;
        let mana_max = gm.pm.current_player.stats.all_stats[MANA].max;
        let old_mana = gm.pm.current_player.stats.all_stats[MANA].current;

        gm.launch_attack(Some("Don de vie"));

        let new_azrak_hp = gm
            .pm
            .get_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let new_elara_hp = gm
            .pm
            .get_active_hero_character(elara_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let new_mana = gm
            .pm
            .get_active_hero_character(elara_id)
            .unwrap()
            .stats
            .all_stats[MANA]
            .current;

        // DecreasingRateOnTurn(3): minimum 1 apply (first roll is always 100%)
        let min_heal_one_apply = 30 * azrak_hp_max / 100;
        assert!(new_azrak_hp > 50, "Azrak should be healed above 50 HP");
        assert!(new_azrak_hp <= azrak_hp_max, "Azrak HP must not exceed max");
        assert!(
            new_azrak_hp >= 50 + min_heal_one_apply,
            "Azrak must receive at least 1 apply (30% HP max = {min_heal_one_apply})"
        );

        // Elara takes -15% HP max self-damage (at least 1 apply)
        let min_self_dmg = 15 * elara_hp_max / 100;
        assert!(
            new_elara_hp < old_elara_hp,
            "Elara should take self-damage from Don de vie"
        );
        assert!(
            old_elara_hp - new_elara_hp >= min_self_dmg,
            "Elara self-damage must be at least 1 apply (15% HP max = {min_self_dmg})"
        );

        // Mana cost: 24% of mana_max
        assert_eq!(
            old_mana - 24 * mana_max / 100,
            new_mana,
            "Elara mana cost: 24% of max"
        );
    }

    #[test]
    fn unit_elara_lumiere_curative_3_heroes_3_enemies() {
        use crate::character_mod::rounds_information::AmountType;

        let mut gm = testing_all_characters::dxrpg_game_manager();
        gm.pm.active_heroes.retain(|h| {
            matches!(
                h.id_name.as_str(),
                "Elara_la_guerisseuse_de_la_Lorien_#1" | "Azrak_Ombresang_#1" | "Thalia_#1"
            )
        });
        gm.pm.active_bosses.truncate(3);

        let elara_id = "Elara_la_guerisseuse_de_la_Lorien_#1";
        let azrak_id = "Azrak_Ombresang_#1";

        let elara = gm.pm.get_active_hero_character(elara_id).unwrap().clone();
        gm.pm.current_player = elara;
        gm.pm.current_player.level = 100;

        // Sub-case A: condition not met (turn 0, no DamageTx on previous turn)
        let lumiere_atk = gm
            .pm
            .current_player
            .attacks_list
            .get("Lumiere curative")
            .unwrap()
            .clone();
        assert!(
            !gm.pm.current_player.can_be_launched(&lumiere_atk, 0),
            "Lumiere curative must be blocked when no prior damage (turn 0)"
        );
        assert!(
            !gm.pm.current_player.can_be_launched(&lumiere_atk, 1),
            "Lumiere curative must be blocked when DamageTx[0] is absent"
        );

        // Sub-case B: condition met — inject DamageTx[1] = 200 and set turn 2
        gm.pm.current_player.character_rounds_info.tx_rx[AmountType::DamageTx as usize]
            .insert(1, 200);
        gm.game_state.current_turn_nb = 2;
        assert!(
            gm.pm.current_player.can_be_launched(&lumiere_atk, 2),
            "Lumiere curative must be launchable when DamageTx[prev_turn] > 0"
        );

        // Drain Azrak to 10 HP and mark as ally target
        gm.pm
            .get_mut_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .current = 10;
        gm.pm
            .get_mut_active_hero_character(azrak_id)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;

        let azrak_hp_max = gm
            .pm
            .get_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .max;
        let mana_max = gm.pm.current_player.stats.all_stats[MANA].max;
        let old_mana = gm.pm.current_player.stats.all_stats[MANA].current;
        let mana_regen = gm.pm.current_player.stats.all_stats[MANA_REGEN].current;
        // Heal formula adds launcher magical power: full_amount = (buffer.value + pow_current)
        let elara_mag_pow = gm.pm.current_player.stats.get_power_stat(true);
        let heal_amount = (130 + elara_mag_pow) as u64;

        gm.launch_attack(Some("Lumiere curative"));

        let new_azrak_hp = gm
            .pm
            .get_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let new_mana = gm
            .pm
            .get_active_hero_character(elara_id)
            .unwrap()
            .stats
            .all_stats[MANA]
            .current;

        // Heal = 130 + Elara's magical power (heal formula: buffer.value + pow_current), capped at max
        let expected_hp = std::cmp::min(10 + heal_amount, azrak_hp_max);
        assert_eq!(
            expected_hp, new_azrak_hp,
            "Azrak must be healed for {} HP (capped at {azrak_hp_max})",
            heal_amount
        );

        // Mana cost 15% of max; eval_end_of_round applies regen so account for mana_regen
        let expected_mana =
            (old_mana as i64 - (15 * mana_max / 100) as i64 + mana_regen as i64) as u64;
        assert_eq!(expected_mana, new_mana, "Elara mana: cost 15% + regen");
    }

    #[test]
    fn unit_elara_non_sans_raison_3_heroes_3_enemies() {
        let mut gm = testing_all_characters::dxrpg_game_manager();
        gm.pm.active_heroes.retain(|h| {
            matches!(
                h.id_name.as_str(),
                "Elara_la_guerisseuse_de_la_Lorien_#1" | "Azrak_Ombresang_#1" | "Thalia_#1"
            )
        });
        gm.pm.active_bosses.truncate(3);

        let elara_id = "Elara_la_guerisseuse_de_la_Lorien_#1";
        let azrak_id = "Azrak_Ombresang_#1";
        let thalia_id = "Thalia_#1";

        let elara = gm.pm.get_active_hero_character(elara_id).unwrap().clone();
        gm.pm.current_player = elara;
        gm.pm.current_player.level = 100;

        // Drain all heroes to 10 HP (zone heal must restore all to max)
        gm.pm.current_player.stats.all_stats[HP].current = 10;
        for h in gm.pm.active_heroes.iter_mut() {
            h.stats.all_stats[HP].current = 10;
        }

        let elara_hp_max = gm.pm.current_player.stats.all_stats[HP].max;
        let azrak_hp_max = gm
            .pm
            .get_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .max;
        let thalia_hp_max = gm
            .pm
            .get_active_hero_character(thalia_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .max;
        let old_mana = gm.pm.current_player.stats.all_stats[MANA].current;

        gm.launch_attack(Some("Non sans raison"));

        // All heroes healed to max HP (100% of HP max heal > remaining 90% deficit)
        let new_elara_hp = gm
            .pm
            .get_active_hero_character(elara_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let new_azrak_hp = gm
            .pm
            .get_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let new_thalia_hp = gm
            .pm
            .get_active_hero_character(thalia_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        assert_eq!(elara_hp_max, new_elara_hp, "Elara must be healed to max HP");
        assert_eq!(azrak_hp_max, new_azrak_hp, "Azrak must be healed to max HP");
        assert_eq!(
            thalia_hp_max, new_thalia_hp,
            "Thalia must be healed to max HP"
        );

        // Elara has BlockHealAtk active (heals blocked for 3 turns)
        assert!(
            gm.pm
                .get_active_hero_character(elara_id)
                .unwrap()
                .character_rounds_info
                .is_heal_atk_blocked,
            "Elara heal attacks must be blocked after Non sans raison"
        );

        // Mana cost: 0 (Non sans raison is free)
        assert_eq!(
            old_mana,
            gm.pm
                .get_active_hero_character(elara_id)
                .unwrap()
                .stats
                .all_stats[MANA]
                .current,
            "Non sans raison has no mana cost"
        );
    }

    #[test]
    fn unit_thrain_enchainement_furieux_3_heroes_3_enemies() {
        let mut gm = testing_all_characters::dxrpg_game_manager();
        gm.pm.active_heroes.retain(|h| {
            matches!(
                h.id_name.as_str(),
                "Thraïn_#1" | "Elara_la_guerisseuse_de_la_Lorien_#1" | "Azrak_Ombresang_#1"
            )
        });
        gm.pm.active_bosses.truncate(3);

        let thrain_id = "Thraïn_#1";
        let thrain = gm.pm.get_active_hero_character(thrain_id).unwrap().clone();
        gm.pm.current_player = thrain;
        gm.pm.current_player.level = 100;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;

        // Set berserk to 60; the attack costs 20% of max berserk per apply and fires as many
        // times as possible (RepeatAsManyAsPossible) until rage is exhausted.
        // actual_cost = raw_cost * berserk_max / 100; nb_applies = floor(60 / actual_cost).
        gm.pm.current_player.stats.all_stats[BERSERK].current = 60;
        let berserk_max = gm.pm.current_player.stats.all_stats[BERSERK].max;
        let berseck_cost = 20u64;
        let actual_cost = (berseck_cost * berserk_max / 100).max(1);
        let nb_applies = (60u64 / actual_cost).max(1);

        // Target: first boss, zero dodge and DamageRxPercent for clean formula
        let boss_id = gm.pm.active_bosses[0].id_name.clone();
        let old_boss_hp = gm.pm.active_bosses[0].stats.all_stats[HP].current;
        gm.pm.active_bosses[0].stats.all_stats[DODGE].current = 0;
        gm.pm.active_bosses[0]
            .character_rounds_info
            .is_current_target = true;
        if let Some(buf) = gm.pm.active_bosses[0]
            .character_rounds_info
            .get_mut_buffer_by_type(&BufKinds::DamageRxPercent)
        {
            buf.value = 0;
        }

        gm.launch_attack(Some("Enchaînement Furieux"));

        // RepeatAsManyAsPossible bypasses armor; each apply deals the raw 50 damage.
        let expected_dmg = nb_applies * 50;
        let new_boss_hp = gm
            .pm
            .get_active_boss_character(&boss_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        assert_eq!(
            old_boss_hp - expected_dmg,
            new_boss_hp,
            "boss HP should drop by {nb_applies} × 50 = {expected_dmg} (RepeatAsManyAsPossible)"
        );

        // Every apply drains rage: total cost = nb_applies × actual_cost.
        let new_berserk = gm
            .pm
            .get_active_hero_character(thrain_id)
            .unwrap()
            .stats
            .all_stats[BERSERK]
            .current;
        let expected_berserk = 60u64.saturating_sub(nb_applies * actual_cost);
        assert_eq!(
            expected_berserk, new_berserk,
            "Thraïn berserk: {nb_applies} applies × {actual_cost} cost each, expected {expected_berserk}"
        );
    }

    #[test]
    fn unit_thrain_provocation_feroce_3_heroes_3_enemies() {
        let mut gm = testing_all_characters::dxrpg_game_manager();
        gm.pm.active_heroes.retain(|h| {
            matches!(
                h.id_name.as_str(),
                "Thraïn_#1" | "Elara_la_guerisseuse_de_la_Lorien_#1" | "Azrak_Ombresang_#1"
            )
        });
        gm.pm.active_bosses.truncate(3);

        let thrain_id = "Thraïn_#1";
        let thrain = gm.pm.get_active_hero_character(thrain_id).unwrap().clone();
        gm.pm.current_player = thrain;
        gm.pm.current_player.level = 100;

        let old_berserk = gm.pm.current_player.stats.all_stats[BERSERK].current;
        let old_crit_max = gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].max;
        let old_aggro = gm.pm.current_player.stats.all_stats[AGGRO].current;

        // Init aggro tx_rx slot so process_aggro can update the stat
        gm.pm
            .current_player
            .init_aggro_on_turn(gm.game_state.current_turn_nb);

        gm.launch_attack(Some("Provocation Féroce "));

        let thrain_after = gm.pm.get_active_hero_character(thrain_id).unwrap();

        // +12 Berserk (free attack, no cost)
        assert_eq!(
            old_berserk + 12,
            thrain_after.stats.all_stats[BERSERK].current,
            "Thraïn berserk must increase by 12 (no cost)"
        );

        // +10 Aggro on self
        assert_eq!(
            old_aggro + 10,
            thrain_after.stats.all_stats[AGGRO].current,
            "Thraïn aggro must increase by 10"
        );

        // +40 max Critical strike for 3 turns
        assert_eq!(
            old_crit_max + 40,
            thrain_after.stats.all_stats[CRITICAL_STRIKE].max,
            "Thraïn critical strike max must increase by 40"
        );

        // 5-turn cooldown applied
        let cooldown_active = thrain_after
            .character_rounds_info
            .all_effects
            .iter()
            .any(|e| {
                e.processed_effect_param.input_effect_param.buffer.kind
                    == BufKinds::CooldownTurnsNumber
                    && e.atk_type.name.contains("Provocation")
            });
        assert!(
            cooldown_active,
            "Provocation Féroce must have a 5-turn cooldown"
        );
    }

    #[test]
    fn unit_thrain_tourbillon_destructeur_3_heroes_3_enemies() {
        let mut gm = testing_all_characters::dxrpg_game_manager();
        gm.pm.active_heroes.retain(|h| {
            matches!(
                h.id_name.as_str(),
                "Thraïn_#1" | "Elara_la_guerisseuse_de_la_Lorien_#1" | "Azrak_Ombresang_#1"
            )
        });
        gm.pm.active_bosses.truncate(3);

        let thrain_id = "Thraïn_#1";
        let thrain = gm.pm.get_active_hero_character(thrain_id).unwrap().clone();
        gm.pm.current_player = thrain;
        gm.pm.current_player.level = 100;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;

        let thrain_phy_pow = gm.pm.current_player.stats.get_power_stat(false);
        let old_berserk = gm.pm.current_player.stats.all_stats[BERSERK].current;
        let berserk_max = gm.pm.current_player.stats.all_stats[BERSERK].max;
        let berseck_cost = 15u64;
        let cost_deducted = berseck_cost * berserk_max / 100;

        let old_berserk_rate_pct =
            gm.pm.current_player.stats.all_stats[BERSECK_RATE].buf_effect_percent;

        // Zero out dodge and DamageRxPercent on all 3 bosses for clean damage formula
        let old_boss_hps: Vec<u64> = gm
            .pm
            .active_bosses
            .iter()
            .map(|b| b.stats.all_stats[HP].current)
            .collect();
        let boss_phy_armors: Vec<i64> = gm
            .pm
            .active_bosses
            .iter()
            .map(|b| b.stats.get_armor_stat(false))
            .collect();
        for boss in gm.pm.active_bosses.iter_mut() {
            boss.stats.all_stats[DODGE].current = 0;
            if let Some(buf) = boss
                .character_rounds_info
                .get_mut_buffer_by_type(&BufKinds::DamageRxPercent)
            {
                buf.value = 0;
            }
        }

        // Init aggro slot so the +5 Aggro self-effect can be recorded
        gm.pm
            .current_player
            .init_aggro_on_turn(gm.game_state.current_turn_nb);
        let old_aggro = gm.pm.current_player.stats.all_stats[AGGRO].current;

        gm.launch_attack(Some("Tourbillon Destructeur "));

        // All 3 bosses take physical damage: raw = -(60 + phy_pow), effective after armor
        for (i, (old_hp, phy_armor)) in old_boss_hps.iter().zip(boss_phy_armors.iter()).enumerate()
        {
            let boss_id = gm.pm.active_bosses[i].id_name.clone();
            let new_hp = gm
                .pm
                .get_active_boss_character(&boss_id)
                .unwrap()
                .stats
                .all_stats[HP]
                .current;
            let raw_dmg = (60_i64 + thrain_phy_pow) as f64;
            let protection =
                AttackType::ARMOR_FACTOR / (AttackType::ARMOR_FACTOR + *phy_armor as f64);
            let expected_dmg = (raw_dmg * protection).round() as i64;
            // HP is floored at 0 when damage exceeds current HP
            let expected_hp = (*old_hp as i64 - expected_dmg).max(0);
            assert_eq!(
                expected_hp, new_hp as i64,
                "boss[{i}] HP must drop by {expected_dmg} (physical zone damage)"
            );
        }

        let thrain_after = gm.pm.get_active_hero_character(thrain_id).unwrap();

        // Berserk cost: 15% of max
        assert_eq!(
            old_berserk - cost_deducted,
            thrain_after.stats.all_stats[BERSERK].current,
            "Thraïn berserk: 15% of max deducted"
        );

        // +5 explicit Aggro on self; zone damage also generates implicit aggro
        assert!(
            thrain_after.stats.all_stats[AGGRO].current >= old_aggro + 5,
            "Thraïn aggro must increase by at least 5 (explicit self effect)"
        );

        // +100% max Berserk rate for 4 turns: buf_effect_percent increased by 100
        assert_eq!(
            old_berserk_rate_pct + 100,
            thrain_after.stats.all_stats[BERSECK_RATE].buf_effect_percent,
            "Berserk rate buf_effect_percent must increase by 100"
        );
    }
}
