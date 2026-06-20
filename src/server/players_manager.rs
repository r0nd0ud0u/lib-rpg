use std::collections::HashMap;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    character_mod::{
        attack_type::AttackType,
        buffers::BufKinds,
        character::{Character, CharacterKind},
        effect::{EffectOutcome, ProcessedEffectParam},
        equipment::{Equipment, EquipmentJsonKey},
        inventory::Consumable,
    },
    common::{
        constants::{
            all_target_const::{TARGET_ALL_ALLIES, TARGET_ALLY, TARGET_ENNEMY, TARGET_HIMSELF},
            character_const::*,
            reach_const::{INDIVIDUAL, ZONE},
            stats_const::*,
        },
        log_data::{LogData, const_colors::LIGHT_GREEN},
    },
    server::game_state::GameState,
};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameAtkEffect {
    pub processed_effect_param: ProcessedEffectParam,
    pub atk_type: AttackType,
    pub launching_turn: usize,
    pub launching_round: usize,
    pub effect_outcome: EffectOutcome,
}

impl GameAtkEffect {
    /// Returns the text line for the attack log, or `None` if this effect should be hidden.
    pub fn log_text(&self) -> Option<String> {
        let kind = &self.processed_effect_param.input_effect_param.buffer.kind;
        let target = &self.effect_outcome.target_id_name;
        let amount = self.effect_outcome.real_amount_tx;
        let full = self.effect_outcome.full_amount_tx;
        let stat = &self
            .processed_effect_param
            .input_effect_param
            .buffer
            .stats_name;
        let nb_turns = self.processed_effect_param.input_effect_param.nb_turns;
        let number_of_applies = self.processed_effect_param.number_of_applies;
        let buf_value = self.processed_effect_param.input_effect_param.buffer.value;

        match kind {
            BufKinds::CooldownTurnsNumber => {
                Some(format!("Cooldown on {target}: {nb_turns} turns"))
            }
            BufKinds::ConditionDamagePrevTurn => {
                if number_of_applies > 0 {
                    Some(format!("{target} → ✓ Condition: damage last turn"))
                } else {
                    Some(format!(
                        "{target} → ✗ Condition: damage last turn (attack stopped)"
                    ))
                }
            }
            BufKinds::MultiValue => Some(format!("{target} → Heal ×{buf_value}")),
            BufKinds::RemoveOneDebuf => {
                if self.effect_outcome.debuff_removed {
                    Some(format!("{target} → debuff removed"))
                } else {
                    None
                }
            }
            BufKinds::ReinitBuf => {
                if stat.is_empty() {
                    None
                } else {
                    Some(format!("{target} → {stat} effects reset"))
                }
            }
            BufKinds::BoostHotsByPercentage => {
                if full > 0 {
                    Some(format!("{target} → HOTs +{buf_value}% (+{full} HP/turn)"))
                } else {
                    Some(format!("{target} → HOTs +{buf_value}%"))
                }
            }
            BufKinds::BoostBufByHotsNumberInPercentage => Some(format!(
                "{target} → +{buf_value}% heal boost per active HOT"
            )),
            _ => {
                if stat == HP
                    && *kind != BufKinds::ChangeMaxStatByPercentage
                    && *kind != BufKinds::ChangeMaxStatByValue
                {
                    if full == amount {
                        Some(format!("{target} → {amount} HP"))
                    } else {
                        Some(format!("{target} → {amount} HP (raw: {full})"))
                    }
                } else if *kind == BufKinds::ChangeMaxStatByPercentage {
                    Some(format!("{target} → {stat} max +{full}%"))
                } else if stat.is_empty() {
                    Some(format!("{target} → {full} ({kind})"))
                } else {
                    Some(format!("{target} → {stat} {full} ({kind})"))
                }
            }
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DodgeInfo {
    pub name: String,
    pub is_dodging: bool,
    pub is_blocking: bool,
}

/// Define all the parameters of a playerManager
/// Should store all the relative data to all the playABLE characters
/// /// Should store all the relative data to all the playING characters
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerManager {
    /// List of all selected heroes by players
    pub active_heroes: Vec<Character>,
    /// List of all selected bosses by computer
    pub active_bosses: Vec<Character>,
    /// Full roster of all bosses loaded from the data manager.
    /// Used as a source when populating active_bosses for each new scenario.
    pub all_bosses: Vec<Character>,
    /// Shadow current player used to update the active character in the list of active characters
    pub current_player: Character,
    /// Equipment table mapping character names to their equipped items
    pub equipment_table: HashMap<EquipmentJsonKey, Vec<Equipment>>,
    /// Shared party consumables pool — available to any hero, consumed when used
    #[serde(default)]
    pub party_consumables: Vec<Consumable>,
}

impl PlayerManager {
    /// Try to create a new PlayerManager by loading all the characters
    /// and by initializing the active heroes with all the loaded heroes
    /// if `default_active_characters` is true.
    /// Bosses are always active by default.
    /// `path` is the root path of the offline directory containing characters and equipments directories.
    pub fn new(equipment_table: HashMap<EquipmentJsonKey, Vec<Equipment>>) -> PlayerManager {
        PlayerManager {
            active_heroes: Vec::new(),
            active_bosses: Vec::new(),
            all_bosses: Vec::new(),
            current_player: Character::default(),
            equipment_table,
            party_consumables: Vec::new(),
        }
    }

    pub fn clear_scenario(&mut self) {
        self.active_bosses.clear();
        self.current_player = Character::default();
        self.active_heroes.iter_mut().for_each(|c| {
            // Reverse active ChangeMaxStat* effects before clearing so buf_effect_*
            // fields are reset to zero and the next scenario starts from the correct base.
            c.reset_all_effects_on_player()
                .expect("failed to reset all effects");
            // Clamp any stat inflated above its max by uncapped passive boosts
            // (e.g. OverHealBoostStat writes directly to stat.current without a cap).
            for stat in c.stats.all_stats.values_mut() {
                stat.current = stat.current.min(stat.max);
            }
            c.character_rounds_info.clear();
            c.stats.get_mut_value(HP).current = c.stats.all_stats[HP].max;
            c.stats.get_mut_value(MANA).current = c.stats.all_stats[MANA].max;
            c.stats.get_mut_value(VIGOR).current = c.stats.all_stats[VIGOR].max;
            c.stats.get_mut_value(BERSERK).current = 0;
            c.stats.get_mut_value(SPEED).current = 0;
            // Reset displayed aggro so the new scenario starts from 0.
            if let Some(aggro) = c.stats.all_stats.get_mut(AGGRO) {
                aggro.current = 0;
            }
        });
    }

    /// Get the number of active heroes with the given name
    pub fn get_nb_of_active_heroes_by_name(&self, db_full_name: &str) -> usize {
        self.active_heroes
            .iter()
            .filter(|c| c.db_full_name == db_full_name)
            .count()
    }

    /// Get the number of active bosses with the given name
    pub fn get_nb_of_active_bosses_by_name(&self, db_full_name: &str) -> usize {
        self.active_bosses
            .iter()
            .filter(|c| c.db_full_name == db_full_name)
            .count()
    }

    pub fn increment_counter_effect(&mut self) {
        for c in self.active_heroes.iter_mut() {
            c.character_rounds_info.increment_counter_effect();
        }
        for c in self.active_bosses.iter_mut() {
            c.character_rounds_info.increment_counter_effect();
        }
    }

    /// The boolean is_first_round is reset for all the characters of the game.
    pub fn reset_is_first_round(&mut self) {
        for c in &mut self.active_heroes {
            c.character_rounds_info.is_first_round = true;
        }
        for c in &mut self.active_bosses {
            c.character_rounds_info.is_first_round = true;
        }
    }

    pub fn apply_regen_stats(&mut self, kind: CharacterKind) {
        let player_list = if kind == CharacterKind::Hero {
            &mut self.active_heroes
        } else {
            &mut self.active_bosses
        };
        for pl in player_list {
            if pl.stats.is_dead().unwrap_or(false) {
                continue;
            }

            pl.stats.apply_regen();
        }
    }

    pub fn get_all_active_id_names(&self) -> Vec<String> {
        let mut output = vec![];
        for h in &self.active_heroes {
            output.push(h.id_name.clone());
        }
        for b in &self.active_bosses {
            output.push(b.id_name.clone());
        }
        output
    }

    pub fn get_mut_active_character(&mut self, id_name: &str) -> Option<&mut Character> {
        if let Some(hero) = self.active_heroes.iter_mut().find(|c| c.id_name == id_name) {
            return Some(hero);
        }
        if let Some(boss) = self.active_bosses.iter_mut().find(|c| c.id_name == id_name) {
            return Some(boss);
        }
        None
    }

    pub fn get_active_character(&self, id_name: &str) -> Option<&Character> {
        if let Some(hero) = self.get_active_hero_character(id_name) {
            return Some(hero);
        }
        if let Some(boss) = self.get_active_boss_character(id_name) {
            return Some(boss);
        }
        None
    }

    pub fn modify_active_character(&mut self, id_name: &str) {
        let pl = self.current_player.clone();
        if let Some(hero) = self.get_mut_active_hero_character(id_name) {
            *hero = pl; // Modify the value inside self.active_heroes
        } else if let Some(boss) = self.get_mut_active_boss_character(id_name) {
            *boss = pl;
        }
    }

    pub fn get_mut_active_hero_character(&mut self, id_name: &str) -> Option<&mut Character> {
        self.active_heroes.iter_mut().find(|c| c.id_name == id_name)
    }

    /// Use a consumable from the shared party bag for the hero identified by `hero_id_name`.
    /// The consumable is removed from the party pool and applied to the hero.
    /// Returns an error if the hero or consumable is not found.
    pub fn use_party_consumable(
        &mut self,
        hero_id_name: &str,
        potion_name: &str,
        game_state: &GameState,
    ) -> Result<()> {
        let idx = self
            .party_consumables
            .iter()
            .position(|c| c.name == potion_name)
            .ok_or_else(|| anyhow::anyhow!("Party consumable '{}' not found", potion_name))?;
        let consumable = self.party_consumables.remove(idx);
        let hero = self
            .get_mut_active_hero_character(hero_id_name)
            .ok_or_else(|| anyhow::anyhow!("Hero '{}' not found", hero_id_name))?;
        let launcher_stats = hero.stats.clone();
        hero.apply_consumable_effects(&consumable, game_state, &launcher_stats)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }

    pub fn get_mut_active_boss_character(&mut self, id_name: &str) -> Option<&mut Character> {
        self.active_bosses.iter_mut().find(|c| c.id_name == id_name)
    }

    pub fn get_active_hero_character(&self, id_name: &str) -> Option<&Character> {
        self.active_heroes.iter().find(|c| c.id_name == id_name)
    }

    pub fn get_active_boss_character(&self, id_name: &str) -> Option<&Character> {
        self.active_bosses.iter().find(|c| c.id_name == id_name)
    }

    pub fn update_current_player_on_new_round(
        &mut self,
        game_state: &GameState,
        id_name: &str,
    ) -> Result<Vec<LogData>> {
        let logs;
        match self.get_mut_active_character(id_name) {
            Some(c) => {
                self.current_player = c.clone();

                // update the shadow current player
                logs = self.current_player.new_round(
                    game_state.current_turn_nb,
                    self.process_launchable_atks(game_state.current_turn_nb),
                );

                // update the active character
                self.modify_active_character(id_name);

                Ok(logs)
            }
            None => {
                bail!("Character '{}' not found", id_name)
            }
        }
    }

    /// Fires the `IsDamageTxHealNeedyAlly` passive for `launcher_id_name` if enabled:
    /// heals the most-needy alive hero by `pct`% of `damage_tx`.
    /// Called immediately after an attack deals damage so the heal appears in the same turn's log.
    pub(crate) fn apply_damage_tx_heal_passive(
        &mut self,
        launcher_id_name: &str,
        damage_tx: i64,
    ) -> Vec<LogData> {
        let mut logs = Vec::new();

        if damage_tx <= 0 {
            return logs;
        }

        let pct = {
            let Some(launcher) = self.get_active_hero_character(launcher_id_name) else {
                return logs;
            };
            let Some(buf) = launcher
                .character_rounds_info
                .get_buffer_by_type(&BufKinds::IsDamageTxHealNeedyAlly)
            else {
                return logs;
            };
            if !buf.is_passive || !buf.is_passive_enabled || buf.value <= 0 {
                return logs;
            }
            buf.value
        };

        let heal_amount = (damage_tx * pct / 100) as u64;
        if heal_amount == 0 {
            return logs;
        }

        // Find the alive hero with the lowest HP ratio (current * 10000 / max).
        let target_id = self
            .active_heroes
            .iter()
            .filter(|c| c.stats.is_dead() == Some(false))
            .min_by_key(|c| {
                c.stats
                    .all_stats
                    .get(HP)
                    .filter(|s| s.max > 0)
                    .map(|s| s.current * 10000 / s.max)
                    .unwrap_or(u64::MAX)
            })
            .map(|c| c.id_name.clone());

        let Some(target_id) = target_id else {
            return logs;
        };

        let Some(target) = self
            .active_heroes
            .iter_mut()
            .find(|c| c.id_name == target_id)
        else {
            return logs;
        };
        let short_name = target.short_name.clone();
        let Some(hp) = target.stats.all_stats.get_mut(HP) else {
            return logs;
        };
        let new_hp = (hp.current + heal_amount).min(hp.max);
        let real_heal = new_hp - hp.current;
        hp.current = new_hp;
        if real_heal > 0 {
            logs.push(LogData {
                message: format!(
                    "\u{26a1} Passive: {} \u{2190} +{} HP ({}% of {} damage TX)",
                    short_name, real_heal, pct, damage_tx
                ),
                color: LIGHT_GREEN.to_string(),
            });
        }

        logs
    }

    /// Process the start of a new turn by incrementing counter effects, resetting first round booleans and applying regen stats.
    pub fn start_new_turn(&mut self, is_first_turn: bool) {
        // Increment turn effects
        self.increment_counter_effect();
        // Reset new round boolean for characters
        self.reset_is_first_round();
        // Apply regen stats but not in first turn
        if !is_first_turn {
            self.apply_regen_stats(CharacterKind::Boss);
            self.apply_regen_stats(CharacterKind::Hero);
        }
    }

    pub fn process_sup_atk_turn(&mut self, launcher_type: CharacterKind) -> Vec<String> {
        let player_list = if launcher_type == CharacterKind::Hero {
            &mut self.active_heroes
        } else {
            &mut self.active_bosses
        };
        for pl in player_list {
            if pl.stats.is_dead().unwrap_or(false) {
                continue;
            }
            let speed = pl
                .stats
                .all_stats
                .get(SPEED)
                .map(|s| s.current)
                .unwrap_or(0);
            if speed >= SPEED_THRESHOLD {
                pl.stats.reset_speed();
                return vec![pl.id_name.clone()];
            }
        }
        vec![]
    }

    pub fn process_all_dodging(
        &mut self,
        all_targets: &Vec<String>,
        atk_level: u64,
        kind: &CharacterKind,
    ) {
        for t in all_targets {
            match self.get_mut_active_character(t) {
                Some(c) => {
                    if c.kind != *kind {
                        c.process_dodging(atk_level);
                    }
                }
                _ => continue,
            }
        }
    }

    pub fn process_died_players(&mut self) -> Result<()> {
        // heroes
        for c in self.active_heroes.iter_mut() {
            if c.stats.is_dead() == Some(true) {
                c.reset_all_effects_on_player()?; // now ? works
                c.character_rounds_info.reset_all_buffers();
            }
        }
        Ok(())
    }

    /// Process the boss target at the start of the turn by setting the hero with the highest aggro as current target.
    /// If all heroes are dead, no target is set.
    /// If the current player is a hero, no target is set.
    pub fn process_boss_target(&mut self) {
        if self.current_player.kind == CharacterKind::Hero {
            return;
        }

        self.reset_targeted_character();
        if let Some((max_index, _)) = self
            .active_heroes
            .iter()
            .enumerate()
            .filter(|(_, c)| c.stats.is_dead() == Some(false))
            .max_by_key(|&(_, c)| c.stats.all_stats[AGGRO].current)
        {
            self.active_heroes[max_index]
                .character_rounds_info
                .is_current_target = true;
        }
    }

    /// Apply target choice from UI
    pub fn set_one_target(&mut self, launcher_id_name: &str, atk_name: &str, target_id_name: &str) {
        if let Some(h) = self.get_mut_active_character(launcher_id_name) {
            let Some(atk) = h.attacks_list.iter().find(|a| a.0 == atk_name) else {
                return;
            };
            if atk.1.reach == ZONE {
                return;
            }
            self.reset_targeted_character();
            if let Some(target) = self.get_mut_active_character(target_id_name) {
                target.character_rounds_info.is_current_target = true;
            }
        }
    }

    /// Get the number of current potential targets (used for UI)
    pub fn get_current_target_nb(&self) -> usize {
        self.active_heroes
            .iter()
            .filter(|c| c.character_rounds_info.is_potential_target)
            .count()
            + self
                .active_bosses
                .iter()
                .filter(|c| c.character_rounds_info.is_potential_target)
                .count()
    }

    pub fn whatif_set_targeted_characters(&self, launcher_id_name: &str, atk_name: &str) -> u64 {
        if let Some(launcher) = self.get_active_character(launcher_id_name) {
            let Some(atk) = launcher
                .attacks_list
                .iter()
                .find(|a| a.0 == atk_name)
                .map(|a| a.1.clone())
            else {
                return 0;
            };

            let is_hero_ally = launcher.kind == CharacterKind::Hero && atk.target == TARGET_ALLY;
            let is_boss_ally = launcher.kind == CharacterKind::Boss && atk.target == TARGET_ALLY;
            let is_boss_ennemy =
                launcher.kind == CharacterKind::Boss && atk.target == TARGET_ENNEMY;
            let is_hero_ennemy =
                launcher.kind == CharacterKind::Hero && atk.target == TARGET_ENNEMY;

            // self - atk
            if atk.target == TARGET_HIMSELF {
                return 1;
            }
            // all heroes - atk
            if atk.target == TARGET_ALL_ALLIES {
                let mut nb = 0;
                self.active_heroes.iter().for_each(|c| {
                    if c.stats.is_dead() == Some(false) {
                        nb += 1;
                    }
                });
                return nb;
            }
            // atk on heroes
            if is_boss_ennemy || is_hero_ally {
                return Self::whatif_targets_for_collection(
                    &self.active_heroes,
                    launcher_id_name,
                    &atk,
                    is_hero_ally,
                    is_boss_ennemy,
                );
            }

            // atk on ennemies
            if is_boss_ally || is_hero_ennemy {
                return Self::whatif_targets_for_collection(
                    &self.active_bosses,
                    launcher_id_name,
                    &atk,
                    is_boss_ally,
                    is_hero_ennemy,
                );
            }
        }

        0
    }

    /// Apply potential target choice for UI
    pub fn set_targeted_characters(&mut self, launcher_id_name: &str, atk_name: &str) {
        self.reset_targeted_character();
        self.reset_potential_targeted_character();

        if let Some(launcher) = self.get_mut_active_character(launcher_id_name) {
            let Some(atk) = launcher
                .attacks_list
                .iter()
                .find(|a| a.0 == atk_name)
                .map(|a| a.1.clone())
            else {
                return;
            };

            let is_hero_ally = launcher.kind == CharacterKind::Hero && atk.target == TARGET_ALLY;
            let is_boss_ally = launcher.kind == CharacterKind::Boss && atk.target == TARGET_ALLY;
            let is_boss_ennemy =
                launcher.kind == CharacterKind::Boss && atk.target == TARGET_ENNEMY;
            let is_hero_ennemy =
                launcher.kind == CharacterKind::Hero && atk.target == TARGET_ENNEMY;

            // self - atk
            if atk.target == TARGET_HIMSELF {
                launcher.character_rounds_info.is_current_target = true;
                launcher.character_rounds_info.is_potential_target = true;
                return;
            }
            // all heroes - atk
            if atk.target == TARGET_ALL_ALLIES {
                self.active_heroes.iter_mut().for_each(|c| {
                    if c.stats.is_dead() == Some(false) {
                        c.character_rounds_info.is_potential_target = true;
                        c.character_rounds_info.is_current_target = true;
                    }
                });
                return;
            }
            // atk on heroes
            if is_boss_ennemy || is_hero_ally {
                Self::set_targets_for_collection(
                    &mut self.active_heroes,
                    launcher_id_name,
                    &atk,
                    is_hero_ally,
                    is_boss_ennemy,
                );
            }

            // atk on ennemies
            if is_boss_ally || is_hero_ennemy {
                Self::set_targets_for_collection(
                    &mut self.active_bosses,
                    launcher_id_name,
                    &atk,
                    is_boss_ally,
                    is_hero_ennemy,
                );
            }
        }
    }

    pub fn reset_targeted_character(&mut self) {
        self.active_heroes
            .iter_mut()
            .for_each(|c| c.character_rounds_info.is_current_target = false);
        self.active_bosses
            .iter_mut()
            .for_each(|c| c.character_rounds_info.is_current_target = false);
    }

    pub fn reset_potential_targeted_character(&mut self) {
        self.active_heroes
            .iter_mut()
            .for_each(|c| c.character_rounds_info.is_potential_target = false);
        self.active_bosses
            .iter_mut()
            .for_each(|c| c.character_rounds_info.is_potential_target = false);
    }

    pub fn process_launchable_atks(&self, current_turn_nb: usize) -> Vec<AttackType> {
        // assess potential target
        let mut launchable_attacks = Vec::new();

        for atk in self.current_player.attacks_list.values() {
            let can_be_launched = self.current_player.can_be_launched(atk, current_turn_nb);
            let whatif_nb =
                self.whatif_set_targeted_characters(&self.current_player.id_name, &atk.name);
            if can_be_launched && whatif_nb > 0 {
                launchable_attacks.push(atk.clone());
            }
        }
        launchable_attacks
    }

    /// Helper function to set targets for a given collection of characters
    /// Extracted to avoid code duplication between heroes and bosses targeting
    fn has_resurrect_effect(atk: &AttackType) -> bool {
        atk.all_effects
            .iter()
            .any(|e| e.buffer.kind == BufKinds::Resurrect)
    }

    fn set_targets_for_collection(
        characters: &mut [Character],
        launcher_id_name: &str,
        atk: &AttackType,
        is_ally_condition: bool,
        is_ennemy_condition: bool,
    ) {
        let can_target_dead = is_ally_condition && Self::has_resurrect_effect(atk);
        let mut has_at_least_one_target = false;
        characters
            .iter_mut()
            .filter(|c| {
                let alive_or_targetable = c.stats.is_dead() == Some(false)
                    || (can_target_dead && c.stats.is_dead() == Some(true));
                alive_or_targetable
                    && ((is_ally_condition && c.id_name != launcher_id_name) || is_ennemy_condition)
            })
            .for_each(|c| {
                if !has_at_least_one_target && atk.reach == INDIVIDUAL || atk.reach == ZONE {
                    c.character_rounds_info.is_current_target = true;
                    c.character_rounds_info.is_potential_target = true;
                    has_at_least_one_target = true;
                } else {
                    c.character_rounds_info.is_potential_target = true;
                }
            });
    }

    /// Helper function to set targets for a given collection of characters
    /// Extracted to avoid code duplication between heroes and bosses targeting
    fn whatif_targets_for_collection(
        characters: &[Character],
        launcher_id_name: &str,
        atk: &AttackType,
        is_ally_condition: bool,
        is_ennemy_condition: bool,
    ) -> u64 {
        let can_target_dead = is_ally_condition && Self::has_resurrect_effect(atk);
        let mut has_at_least_one_target = false;
        let mut nb = 0;
        characters
            .iter()
            .filter(|c| {
                let alive_or_targetable = c.stats.is_dead() == Some(false)
                    || (can_target_dead && c.stats.is_dead() == Some(true));
                alive_or_targetable
                    && ((is_ally_condition && c.id_name != launcher_id_name) || is_ennemy_condition)
            })
            .for_each(|_c| {
                if !has_at_least_one_target && atk.reach == INDIVIDUAL || atk.reach == ZONE {
                    nb += 1;
                    has_at_least_one_target = true;
                } else {
                    nb += 1;
                }
            });
        nb
    }

    /// Check if the game is over by checking if all heroes or all bosses are dead.
    /// Returns a tuple of booleans (all_heroes_dead, all_bosses_dead).
    pub fn check_end_of_game(&self) -> (bool, bool) {
        let all_heroes_dead = self
            .active_heroes
            .iter()
            .all(|c| c.stats.is_dead() == Some(true));
        let all_bosses_dead = self
            .active_bosses
            .iter()
            .all(|c| c.stats.is_dead() == Some(true));
        (all_heroes_dead, all_bosses_dead)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        character_mod::{effect::EffectOutcome, equipment::EquipmentJsonKey},
        common::constants::stats_const::*,
        server::game_state::GameState,
        server::players_manager::GameAtkEffect,
        testing::testing_all_characters::{self, testing_pm},
        testing::testing_effect::*,
    };
    use strum::IntoEnumIterator;

    #[test]
    fn unit_try_new() {
        let pl = testing_all_characters::testing_pm();

        // equipments
        assert_eq!(EquipmentJsonKey::iter().count(), pl.equipment_table.len());
    }

    #[test]
    fn unit_all_bosses() {
        use crate::character_mod::character::CharacterKind;
        let pl = testing_all_characters::testing_pm();
        assert!(!pl.all_bosses.is_empty(), "all_bosses should not be empty");
        for b in &pl.all_bosses {
            assert_eq!(
                b.kind,
                CharacterKind::Boss,
                "all_bosses should only contain Boss characters, got {:?}",
                b.db_full_name
            );
        }
    }

    #[test]
    fn unit_increment_counter_effect() {
        let mut pl = testing_all_characters::testing_pm();
        pl.active_heroes[0]
            .character_rounds_info
            .all_effects
            .push(GameAtkEffect {
                processed_effect_param: build_cooldown_effect(),
                ..Default::default()
            });
        let old_counter_turn = pl.active_heroes[0].character_rounds_info.all_effects[0]
            .processed_effect_param
            .counter_turn;
        pl.increment_counter_effect();
        assert_eq!(
            pl.active_heroes[0].character_rounds_info.all_effects[0]
                .processed_effect_param
                .counter_turn,
            old_counter_turn + 1
        );
    }

    #[test]
    fn unit_reset_is_first_round() {
        let mut pl = testing_all_characters::testing_pm();
        pl.reset_is_first_round();
        assert!(pl.active_heroes[0].character_rounds_info.is_first_round);
    }

    #[test]
    fn unit_apply_regen_stats() {
        let mut pl = testing_all_characters::testing_pm();
        let old_hp = pl.active_heroes[0].stats.all_stats[HP].current;
        let hp_regen = pl.active_heroes[0].stats.all_stats[HP_REGEN].current;
        let old_mana = pl.active_heroes[0].stats.all_stats[MANA].current;
        let mana_regen = pl.active_heroes[0].stats.all_stats[MANA_REGEN].current;
        let old_berseck = pl.active_heroes[0].stats.all_stats[BERSERK].current;
        let berseck_regen = pl.active_heroes[0].stats.all_stats[BERSECK_RATE].current;
        let old_vigor = pl.active_heroes[0].stats.all_stats[VIGOR].current;
        let vigor_regen = pl.active_heroes[0].stats.all_stats[VIGOR_REGEN].current;
        let old_speed = pl.active_heroes[0].stats.all_stats[SPEED].current;
        let speed_regen = pl.active_heroes[0].stats.all_stats[SPEED_REGEN].current;
        pl.apply_regen_stats(crate::character_mod::character::CharacterKind::Hero);
        assert_eq!(
            old_hp + hp_regen,
            pl.active_heroes[0].stats.all_stats[HP].current
        );
        assert_eq!(
            std::cmp::min(
                old_mana + mana_regen,
                pl.active_heroes[0].stats.all_stats[MANA].max
            ),
            pl.active_heroes[0].stats.all_stats[MANA].current
        );
        assert_eq!(
            std::cmp::min(
                old_berseck + berseck_regen,
                pl.active_heroes[0].stats.all_stats[BERSERK].max
            ),
            pl.active_heroes[0].stats.all_stats[BERSERK].current
        );
        assert_eq!(
            std::cmp::min(
                old_vigor + vigor_regen,
                pl.active_heroes[0].stats.all_stats[VIGOR].max
            ),
            pl.active_heroes[0].stats.all_stats[VIGOR].current
        );
        assert_eq!(
            old_speed + speed_regen,
            pl.active_heroes[0].stats.all_stats[SPEED].current
        );

        let old_hp = pl.active_bosses[0].stats.all_stats[HP].current;
        let hp_regen = pl.active_bosses[0].stats.all_stats[HP_REGEN].current;
        pl.apply_regen_stats(crate::character_mod::character::CharacterKind::Boss);
        // max is topped
        assert_eq!(
            std::cmp::min(
                pl.active_bosses[0].stats.all_stats[HP].max,
                old_hp + hp_regen
            ),
            pl.active_bosses[0].stats.all_stats[HP].current
        );
    }

    #[test]
    fn unit_get_mut_active_character() {
        let mut pl = testing_all_characters::testing_pm();
        assert!(pl.get_mut_active_character("test_#1").is_some());
        assert!(pl.get_mut_active_character("test_boss1_#1").is_some());
        assert!(pl.get_mut_active_character("unknown").is_none());
    }

    #[test]
    fn unit_get_active_character() {
        let pl = testing_all_characters::testing_pm();
        assert!(pl.get_active_character("test_#1").is_some());
        assert!(pl.get_active_character("test_boss1_#1").is_some());
        assert!(pl.get_active_character("unknown").is_none());
    }

    #[test]
    fn unit_update_current_player() {
        let mut pl = testing_all_characters::testing_pm();
        pl.get_mut_active_hero_character("test_#1")
            .unwrap()
            .character_rounds_info
            .is_first_round = false;
        pl.get_mut_active_hero_character("test_#1")
            .unwrap()
            .character_rounds_info
            .actions_done_in_round = 100;
        let gs = GameState::default();
        pl.update_current_player_on_new_round(&gs, "test_#1")
            .unwrap();
        assert_eq!(
            0,
            pl.get_mut_active_hero_character("test_#1")
                .unwrap()
                .character_rounds_info
                .actions_done_in_round
        );
    }

    #[test]
    fn unit_pl_process_hot_and_dot() {
        let mut pl = testing_all_characters::testing_pm();
        // push default effect
        pl.current_player
            .character_rounds_info
            .all_effects
            .push(GameAtkEffect::default());
        let mut gs = GameState::new();
        let (logs, hot_and_dot) = pl
            .current_player
            .character_rounds_info
            .process_hot_and_dot(gs.current_turn_nb);
        assert_eq!(0, logs.len());
        assert_eq!(0, hot_and_dot);
        // test cooldown effect
        pl.current_player
            .character_rounds_info
            .all_effects
            .push(GameAtkEffect {
                processed_effect_param: build_cooldown_effect(),
                ..Default::default()
            });
        let (logs, hot_and_dot) = pl
            .current_player
            .character_rounds_info
            .process_hot_and_dot(gs.current_turn_nb);
        assert_eq!(0, logs.len());
        assert_eq!(0, hot_and_dot);
        // add test HOT but on same turn
        pl.current_player
            .character_rounds_info
            .all_effects
            .push(GameAtkEffect {
                processed_effect_param: build_hot_effect_individual(),
                effect_outcome: EffectOutcome {
                    full_amount_tx: 30,
                    ..Default::default()
                },
                ..Default::default()
            });
        let (logs, hot_and_dot) = pl
            .current_player
            .character_rounds_info
            .process_hot_and_dot(gs.current_turn_nb);
        assert_eq!(0, logs.len());
        assert_eq!(0, hot_and_dot);
        // add test HOT on different turn
        gs.start_new_turn();
        let (logs, hot_and_dot) = pl
            .current_player
            .character_rounds_info
            .process_hot_and_dot(gs.current_turn_nb);
        assert_eq!(1, logs.len());
        assert_eq!(30, hot_and_dot);
        // add test DOT on different turn
        pl.current_player
            .character_rounds_info
            .all_effects
            .push(GameAtkEffect {
                processed_effect_param: build_dot_effect_individual(),
                effect_outcome: EffectOutcome {
                    full_amount_tx: -20,
                    ..Default::default()
                },
                ..Default::default()
            });
        let (logs, hot_and_dot) = pl
            .current_player
            .character_rounds_info
            .process_hot_and_dot(gs.current_turn_nb);
        assert_eq!(2, logs.len()); // hot + dot
        assert_eq!(10, hot_and_dot); // 30(hot) - 20 (dot)
    }

    #[test]
    fn unit_set_one_target() {
        let mut pl = testing_all_characters::testing_pm();
        // simpleAtk is indiv launched by a boss
        pl.set_one_target("test_boss1_#1", "SimpleAtk", "test_#1");
        assert!(
            pl.get_mut_active_hero_character("test_#1")
                .unwrap()
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_mut_active_boss_character("test_boss1_#1")
                .unwrap()
                .character_rounds_info
                .is_current_target
        );
        // indiv launched a hero
        pl.set_one_target("test_#1", "SimpleAtk", "test_boss1_#1");
        assert!(
            !pl.get_mut_active_hero_character("test_#1")
                .unwrap()
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_mut_active_boss_character("test_boss1_#1")
                .unwrap()
                .character_rounds_info
                .is_current_target
        );
        // whatever launched with ZONE no reset is done
        pl.set_one_target("test_#1", "simple-atk-zone", "test_boss1_#1");
        assert!(
            !pl.get_mut_active_hero_character("test_#1")
                .unwrap()
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_mut_active_boss_character("test_boss1_#1")
                .unwrap()
                .character_rounds_info
                .is_current_target
        );
        pl.set_one_target("test_#1", "Offrande vitale", "test2_#1");
        assert!(
            pl.get_mut_active_hero_character("test2_#1")
                .unwrap()
                .character_rounds_info
                .is_current_target
        );
    }

    #[test]
    fn unit_set_targeted_characters() {
        let mut pl = testing_pm();
        // hero is attacking
        // atk to ennemy - effect dmg indiv
        let test_ally_id_name = "test_#1";
        let test2_ally_id_name = "test2_#1";
        let boss_id_name = "test_boss1_#1";
        let boss2_id_name = "test_boss2_#1";
        pl.get_active_character(test_ally_id_name).expect("no hero");
        pl.set_targeted_characters(test_ally_id_name, "SimpleAtk");
        assert_eq!(2, pl.active_bosses.len());
        let current_nb = pl
            .active_bosses
            .iter_mut()
            .filter(|x| x.character_rounds_info.is_current_target)
            .count();
        assert_eq!(current_nb, 1);
        let potential_nb = pl
            .active_bosses
            .iter_mut()
            .filter(|x| x.character_rounds_info.is_potential_target)
            .count();
        assert_eq!(potential_nb, 2);
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        // atk to ennemy - effect dmg zone
        pl.set_targeted_characters(test_ally_id_name, "simple-atk-zone");
        assert!(
            pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        // atk to ally(himself in this example) - effect heal indiv, test -> test2
        pl.set_targeted_characters(test_ally_id_name, "simple-atk-himself");
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target,
        );
        assert!(
            pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target,
        );
        assert!(
            !pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        // atk to ally(himself in this example) - effect heal indiv, test2 -> test
        pl.set_targeted_characters(test2_ally_id_name, "simple-atk-himself");
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        // atk to ally - effect heal zone
        pl.set_targeted_characters(test_ally_id_name, "simple-atk-ally-zone");
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        // atk to all heroes target
        pl.set_targeted_characters(test_ally_id_name, "simple-atk-all-heroes");
        let current_nb = pl
            .active_heroes
            .iter_mut()
            .filter(|x| x.character_rounds_info.is_current_target)
            .count();
        assert_eq!(current_nb, 2);
        let potential_nb = pl
            .active_heroes
            .iter_mut()
            .filter(|x| x.character_rounds_info.is_potential_target)
            .count();
        assert_eq!(potential_nb, 2);

        // boss is attacking
        // atk from ennemy - effect dmg indiv
        pl.set_targeted_characters(boss_id_name, "SimpleAtk");
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        let nb = pl
            .active_heroes
            .iter_mut()
            .filter(|x| x.character_rounds_info.is_current_target)
            .count();
        assert_eq!(nb, 1);
        assert!(
            pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target,
        );
        // atk from ennemy - effect dmg zone
        pl.set_targeted_characters(boss_id_name, "simple-atk-zone");
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        // atk to ally(himself in this example) - effect heal indiv
        pl.set_targeted_characters(boss_id_name, "simple-atk-himself");
        assert!(
            pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        // boss atk to ally - effect heal zone  => ZONE is not himself
        pl.set_targeted_characters(boss_id_name, "simple-atk-ally-zone");
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            pl.get_active_character(boss2_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_active_character(boss2_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
    }

    #[test]
    fn unit_set_targeted_characters_test_dead_character() {
        let mut pl = testing_pm();
        // hero is attacking
        // atk to ennemy - effect dmg indiv
        let test_ally_id_name = "test_#1";
        let boss_id_name = "test_boss1_#1";
        let boss2_id_name = "test_boss2_#1";
        pl.get_active_character(test_ally_id_name).expect("no hero");
        pl.get_mut_active_character(boss_id_name)
            .expect("no boss")
            .stats
            .all_stats
            .get_mut(HP)
            .unwrap()
            .current = 0; // boss is dead
        pl.set_targeted_characters(test_ally_id_name, "SimpleAtk");
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        // consequently only one boss remaining, that boss is the target
        assert!(
            pl.get_active_character(boss2_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_active_character(boss2_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
    }

    #[test]
    fn unit_get_nb_of_active_heroes_by_name() {
        let pl = testing_all_characters::testing_pm();
        assert_eq!(1, pl.get_nb_of_active_heroes_by_name("test"));
        assert_eq!(0, pl.get_nb_of_active_heroes_by_name("unknown"));
    }

    #[test]
    fn unit_get_nb_of_active_bosses_by_name() {
        let pl = testing_all_characters::testing_pm();
        assert_eq!(1, pl.get_nb_of_active_bosses_by_name("test_boss1"));
        assert_eq!(0, pl.get_nb_of_active_bosses_by_name("unknown"));
    }

    #[test]
    fn unit_get_current_target_nb() {
        let mut pl = testing_all_characters::testing_pm();
        assert_eq!(0, pl.get_current_target_nb());
        pl.active_heroes[0]
            .character_rounds_info
            .is_potential_target = true;
        assert_eq!(1, pl.get_current_target_nb());
        pl.active_bosses[0]
            .character_rounds_info
            .is_potential_target = true;
        assert_eq!(2, pl.get_current_target_nb());
    }

    #[test]
    fn unit_whatif_set_targeted_characters() {
        let pl = testing_all_characters::testing_pm();
        // hero is attacking
        // atk to ennemy - effect dmg indiv
        let test_ally_id_name = "test_#1";
        pl.get_active_character(test_ally_id_name).expect("no hero");
        let potential_target_nb = pl.whatif_set_targeted_characters(test_ally_id_name, "SimpleAtk");
        assert_eq!(2, potential_target_nb);
        // atk to ennemy - effect dmg zone
        let potential_target_nb =
            pl.whatif_set_targeted_characters(test_ally_id_name, "simple-atk-zone");
        assert_eq!(2, potential_target_nb);
        // atk to ally(himself in this example) - effect heal indiv, test -> test2
        let potential_target_nb =
            pl.whatif_set_targeted_characters(test_ally_id_name, "simple-atk-himself");
        assert_eq!(1, potential_target_nb);
    }

    #[test]
    fn unit_use_party_consumable_ok_and_err() {
        use crate::character_mod::inventory::Consumable;
        let mut pl = testing_pm();

        // error: consumable not found
        assert!(
            pl.use_party_consumable("test_#1", "NoSuchPotion", &GameState::default())
                .is_err()
        );

        // success: valid hero + valid potion
        pl.party_consumables.push(Consumable {
            name: "TestPotion".to_string(),
            ..Default::default()
        });
        assert!(
            pl.use_party_consumable("test_#1", "TestPotion", &GameState::default())
                .is_ok()
        );
        // consumable removed after use
        assert!(pl.party_consumables.is_empty());

        // error: hero not found (consumable removed by the function before hero check)
        pl.party_consumables.push(Consumable {
            name: "TestPotion2".to_string(),
            ..Default::default()
        });
        assert!(
            pl.use_party_consumable("no_hero", "TestPotion2", &GameState::default())
                .is_err()
        );
    }

    #[test]
    fn unit_process_launchable_atks() {
        let mut pl = testing_all_characters::testing_pm();
        // no problem of level
        pl.current_player.level = 100;
        // no problem of is_heal_atk_blocked
        pl.current_player.character_rounds_info.is_heal_atk_blocked = false;
        let launchable_atks = pl.process_launchable_atks(0);
        assert_eq!(pl.current_player.attacks_list.len(), launchable_atks.len()); // in the list, one is berserk atk type and test.json has not the berserk energy!!

        // case level under
        pl.current_player.level = 1;
        let launchable_atks = pl.process_launchable_atks(0);
        assert_eq!(13, launchable_atks.len()); // 13 on 17 are level 1

        // case is_heal_atk_blocked
        pl.current_player.character_rounds_info.is_heal_atk_blocked = true;
        pl.current_player.level = 100;
        let launchable_atks = pl.process_launchable_atks(0);
        assert_eq!(10, launchable_atks.len()); // 6 attacks are HP and linked to is_heal_atk_blocked condition
    }

    /// clear_scenario must reset speed to 0, restore stat maxima inflated by active
    /// ChangeMaxStat* effects (e.g. speed_regen, dodge), and set HP/Mana/Vigor to full.
    #[test]
    fn unit_clear_scenario_resets_stats() {
        use crate::character_mod::{
            buffers::{BufKinds, Buffer},
            effect::{EffectParam, ProcessedEffectParam},
        };
        use crate::common::constants::{all_target_const::TARGET_HIMSELF, reach_const::INDIVIDUAL};

        let mut pm = testing_pm();

        let old_dodge_max = pm.active_heroes[0].stats.all_stats[DODGE].max;
        let old_speed_regen_max = pm.active_heroes[0].stats.all_stats[SPEED_REGEN].max;
        let old_hp_max = pm.active_heroes[0].stats.all_stats[HP].max;
        let old_mana_max = pm.active_heroes[0].stats.all_stats[MANA].max;

        // Inject ChangeMaxStatByValue +20 on Dodge (simulates an active mid-scenario buff).
        let dodge_ep = EffectParam {
            buffer: Buffer {
                kind: BufKinds::ChangeMaxStatByValue,
                value: 20,
                is_percent: false,
                stats_name: DODGE.to_string(),
                ..Default::default()
            },
            nb_turns: 3,
            target_kind: TARGET_HIMSELF.to_string(),
            reach: INDIVIDUAL.to_string(),
            ..Default::default()
        };
        pm.active_heroes[0]
            .character_rounds_info
            .all_effects
            .push(GameAtkEffect {
                processed_effect_param: ProcessedEffectParam {
                    input_effect_param: dodge_ep,
                    number_of_applies: 1,
                    ..Default::default()
                },
                ..Default::default()
            });
        pm.active_heroes[0]
            .stats
            .set_stats_on_effect(DODGE, 20, false, true);

        // Inject ChangeMaxStatByValue +5 on SPEED_REGEN (simulates a speed-regen buff).
        let sr_ep = EffectParam {
            buffer: Buffer {
                kind: BufKinds::ChangeMaxStatByValue,
                value: 5,
                is_percent: false,
                stats_name: SPEED_REGEN.to_string(),
                ..Default::default()
            },
            nb_turns: 3,
            target_kind: TARGET_HIMSELF.to_string(),
            reach: INDIVIDUAL.to_string(),
            ..Default::default()
        };
        pm.active_heroes[0]
            .character_rounds_info
            .all_effects
            .push(GameAtkEffect {
                processed_effect_param: ProcessedEffectParam {
                    input_effect_param: sr_ep,
                    number_of_applies: 1,
                    ..Default::default()
                },
                ..Default::default()
            });
        pm.active_heroes[0]
            .stats
            .set_stats_on_effect(SPEED_REGEN, 5, false, true);

        // Confirm effects are live before the clear.
        assert_eq!(
            old_dodge_max + 20,
            pm.active_heroes[0].stats.all_stats[DODGE].max,
            "pre-check: Dodge should be buffed"
        );
        assert_eq!(
            old_speed_regen_max + 5,
            pm.active_heroes[0].stats.all_stats[SPEED_REGEN].max,
            "pre-check: Speed regen should be buffed"
        );

        // Simulate accumulated speed (e.g. after a supplementary-attack reset_speed call).
        pm.active_heroes[0].stats.get_mut_value(SPEED).current = 50;
        // Partially drain HP and Mana.
        pm.active_heroes[0].stats.get_mut_value(HP).current = 10;
        pm.active_heroes[0].stats.get_mut_value(MANA).current = 5;

        // --- Act ---
        pm.clear_scenario();

        // --- Assert ---
        let hero = &pm.active_heroes[0];

        assert_eq!(
            0, hero.stats.all_stats[SPEED].current,
            "Speed must be 0 after clear_scenario"
        );
        assert_eq!(
            old_dodge_max, hero.stats.all_stats[DODGE].max,
            "Dodge max must be restored after clear_scenario"
        );
        assert_eq!(
            old_speed_regen_max, hero.stats.all_stats[SPEED_REGEN].max,
            "Speed regen max must be restored after clear_scenario"
        );
        assert_eq!(
            old_hp_max, hero.stats.all_stats[HP].current,
            "HP must be restored to max after clear_scenario"
        );
        assert_eq!(
            old_mana_max, hero.stats.all_stats[MANA].current,
            "Mana must be restored to max after clear_scenario"
        );
        assert_eq!(
            0, hero.stats.all_stats[BERSERK].current,
            "Berserk must be 0 after clear_scenario"
        );
        assert_eq!(
            0, hero.stats.all_stats[AGGRO].current,
            "Aggro must be 0 after clear_scenario"
        );
    }

    /// clear_scenario must clamp any stat inflated above max by the OverHealBoostStat passive.
    #[test]
    fn unit_clear_scenario_resets_overheal_passive_stat_boost() {
        use crate::common::constants::stats_const::PHYSICAL_POWER;

        let mut pm = testing_all_characters::dxrpg_pm();
        // Use Azrak who carries the OverHealBoostStat passive on Physical power.
        let azrak_id = "Azrak_Ombresang_#1";
        let phys_pow_max = pm
            .get_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .all_stats[PHYSICAL_POWER]
            .max;

        // Simulate the passive having accumulated a large uncapped boost across a scenario.
        pm.get_mut_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .get_mut_value(PHYSICAL_POWER)
            .current = phys_pow_max + 200;

        pm.clear_scenario();

        let after = pm
            .get_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .all_stats[PHYSICAL_POWER]
            .current;
        assert_eq!(
            after, phys_pow_max,
            "Physical power current must be clamped back to max after clear_scenario"
        );
    }

    #[test]
    fn unit_passive_damage_tx_heal_needy_ally_fires() {
        use crate::character_mod::buffers::Buffer;
        let mut pm = testing_all_characters::testing_pm();
        let launcher_id = pm.active_heroes[0].id_name.clone();

        // Install IsDamageTxHealNeedyAlly passive on hero[0] at 25%
        pm.active_heroes[0]
            .character_rounds_info
            .all_buffers
            .push(Buffer {
                kind: crate::character_mod::buffers::BufKinds::IsDamageTxHealNeedyAlly,
                value: 25,
                is_passive: true,
                is_passive_enabled: true,
                ..Default::default()
            });

        // Set launcher (hero[0]) to full HP so hero[1] (at HP=1) is the most needy
        let max_hp_launcher = pm.active_heroes[0].stats.all_stats[HP].max;
        pm.active_heroes[0]
            .stats
            .all_stats
            .get_mut(HP)
            .unwrap()
            .current = max_hp_launcher;

        // hero[1] starts at HP=1 (from test JSON) — lowest ratio → most needy
        let hero1_hp_before = pm.active_heroes[1].stats.all_stats[HP].current; // 1

        // Passive fires immediately with damage_tx=200 (simulates an attack dealing 200 damage)
        pm.apply_damage_tx_heal_passive(&launcher_id, 200);

        let hero1_hp_after = pm.active_heroes[1].stats.all_stats[HP].current;
        // 200 * 25 / 100 = 50
        assert_eq!(
            hero1_hp_after,
            hero1_hp_before + 50,
            "most needy ally must receive 25% of 200 damage TX"
        );
    }

    #[test]
    fn unit_passive_damage_tx_heal_needy_ally_noop_when_no_damage() {
        use crate::character_mod::buffers::Buffer;
        let mut pm = testing_all_characters::testing_pm();
        let launcher_id = pm.active_heroes[0].id_name.clone();

        pm.active_heroes[0]
            .character_rounds_info
            .all_buffers
            .push(Buffer {
                kind: crate::character_mod::buffers::BufKinds::IsDamageTxHealNeedyAlly,
                value: 25,
                is_passive: true,
                is_passive_enabled: true,
                ..Default::default()
            });

        let hero1_hp_before = pm.active_heroes[1].stats.all_stats[HP].current;

        // damage_tx=0 → passive must be a no-op
        pm.apply_damage_tx_heal_passive(&launcher_id, 0);

        assert_eq!(
            pm.active_heroes[1].stats.all_stats[HP].current, hero1_hp_before,
            "zero damage TX means no passive heal"
        );
    }

    #[test]
    fn unit_passive_damage_tx_heal_needy_ally_disabled() {
        use crate::character_mod::buffers::Buffer;
        let mut pm = testing_all_characters::testing_pm();
        let launcher_id = pm.active_heroes[0].id_name.clone();

        // passive exists but is disabled
        pm.active_heroes[0]
            .character_rounds_info
            .all_buffers
            .push(Buffer {
                kind: crate::character_mod::buffers::BufKinds::IsDamageTxHealNeedyAlly,
                value: 25,
                is_passive: true,
                is_passive_enabled: false, // disabled
                ..Default::default()
            });

        let hero1_hp_before = pm.active_heroes[1].stats.all_stats[HP].current;

        pm.apply_damage_tx_heal_passive(&launcher_id, 200);

        assert_eq!(
            pm.active_heroes[1].stats.all_stats[HP].current, hero1_hp_before,
            "disabled passive must not heal"
        );
    }

    #[test]
    fn unit_passive_damage_tx_heal_needy_ally_capped_at_max_hp() {
        use crate::character_mod::buffers::Buffer;
        let mut pm = testing_all_characters::testing_pm();
        let launcher_id = pm.active_heroes[0].id_name.clone();

        pm.active_heroes[0]
            .character_rounds_info
            .all_buffers
            .push(Buffer {
                kind: crate::character_mod::buffers::BufKinds::IsDamageTxHealNeedyAlly,
                value: 25,
                is_passive: true,
                is_passive_enabled: true,
                ..Default::default()
            });

        // Set launcher (hero[0]) to full HP so hero[1] (at HP=1) is the most needy
        let max_hp_launcher = pm.active_heroes[0].stats.all_stats[HP].max;
        pm.active_heroes[0]
            .stats
            .all_stats
            .get_mut(HP)
            .unwrap()
            .current = max_hp_launcher;

        // hero[1] starts at HP=1 (most needy); track its max for the cap assertion
        let hp_max = pm.active_heroes[1].stats.all_stats[HP].max; // 135

        // Huge damage TX so heal would overflow HP max
        pm.apply_damage_tx_heal_passive(&launcher_id, 10_000);

        assert_eq!(
            pm.active_heroes[1].stats.all_stats[HP].current, hp_max,
            "heal must not exceed HP max"
        );
    }
}
