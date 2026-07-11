use crate::{
    character_mod::{
        attack_type::{AttackType, LauncherAtkInfo},
        buffers::BufKinds,
        rounds_information::AmountType,
    },
    common::{
        constants::stats_const::*,
        log_data::{
            LogData,
            const_colors::{DARK_RED, LIGHT_BLUE, LIGHT_GREEN},
        },
    },
    server::{
        game_state::GameStatus,
        players_manager::{DodgeInfo, GameAtkEffect},
    },
    utils,
};

use super::{GameManager, ResultLaunchAttack};

impl GameManager {
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
        logs_atk.extend(passive_logs.clone());
        let mut result_attack = ResultLaunchAttack {
            launcher_id_name: self.pm.current_player.id_name.clone(),
            atk_name: atk_name.to_string(),
            is_crit,
            new_game_atk_effects: new_game_atk_effects.clone(),
            all_dodging: all_dodging.clone(),
            is_boss_atk: self.pm.current_player.is_boss_atk(),
            logs_end_of_round: Vec::new(),
            logs_atk,
            passive_logs,
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
                let Some(text) = gae.log_text() else {
                    continue;
                };
                let is_hp = gae
                    .processed_effect_param
                    .input_effect_param
                    .buffer
                    .stats_name
                    == HP;
                let is_damage = is_hp
                    && (gae.effect_outcome.real_amount_tx < 0
                        || gae.effect_outcome.full_amount_tx < 0);
                let is_condition_fail = gae.processed_effect_param.input_effect_param.buffer.kind
                    == BufKinds::ConditionDamagePrevTurn
                    && gae.processed_effect_param.number_of_applies == 0;
                let color = if is_damage || is_condition_fail {
                    DARK_RED
                } else {
                    LIGHT_GREEN
                };
                logs.push(LogData {
                    message: text,
                    color: color.to_string(),
                });
            }
        }
        logs
    }
}

#[cfg(test)]
#[path = "azrak_tests.rs"]
mod azrak_tests;
#[cfg(test)]
#[path = "elara_tests.rs"]
mod elara_tests;
#[cfg(test)]
#[path = "thalia_tests.rs"]
mod thalia_tests;
#[cfg(test)]
#[path = "thrain_tests.rs"]
mod thrain_tests;

#[cfg(test)]
mod tests {
    use crate::character_mod::attack_type::AttackType;
    use crate::character_mod::buffers::{BufKinds, Buffer};
    use crate::character_mod::class::Class;
    use crate::character_mod::rank::Rank;
    use crate::common::constants::attak_const::COEFF_CRIT_DMG;
    use crate::common::constants::streak_breaker_const::STREAK_BREAKER_ADVANCED;
    use crate::common::log_data::const_colors::DARK_RED;
    use crate::server::game_manager::LogData;
    use crate::server::game_state::GameStatus;
    use crate::testing::testing_all_characters::{self, testing_test_ally1_vs_test_boss1};
    use crate::{common::constants::stats_const::*, testing::testing_atk::*};

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
        // power_factor = 1 + hero_pow/POWER_SCALE; raw = 35 * power_factor (positive magnitude)
        // defense = armor + boss_pow/DEFENSE_DIVISOR
        let hero_total_pow = gm.pm.current_player.stats.get_power_stat(false);
        let boss_total_armor = old_boss.stats.get_armor_stat(false);
        let boss_total_pow = old_boss.stats.get_power_stat(false);
        let power_factor = 1.0 + hero_total_pow as f64 / AttackType::POWER_SCALE;
        let raw_dmg = (35_f64 * power_factor).round() as i64;
        let defense = boss_total_armor as f64 + boss_total_pow as f64 / AttackType::DEFENSE_DIVISOR;
        let protection = AttackType::ARMOR_FACTOR / (AttackType::ARMOR_FACTOR + defense);
        let atk_amount = (raw_dmg as f64 * protection).round() as i64;
        assert_eq!(
            std::cmp::max(0, old_hp_boss as i64 - atk_amount) as u64,
            gm.pm
                .get_active_boss_character(&target_id_name)
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        // flat cost: 9
        assert_eq!(
            old_vigor_hero - 9,
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
        // flat cost: 9
        assert_eq!(
            old_vigor_hero - 9,
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
        // multiplicative power formula; crit applied on top of armor-mitigated damage
        let hero_total_pow = gm.pm.current_player.stats.get_power_stat(false);
        let boss_total_armor = old_boss.stats.get_armor_stat(false);
        let boss_total_pow = old_boss.stats.get_power_stat(false);
        let power_factor = 1.0 + hero_total_pow as f64 / AttackType::POWER_SCALE;
        let raw_dmg = (35_f64 * power_factor).round() as i64;
        let defense = boss_total_armor as f64 + boss_total_pow as f64 / AttackType::DEFENSE_DIVISOR;
        let protection = AttackType::ARMOR_FACTOR / (AttackType::ARMOR_FACTOR + defense);
        let effective = (raw_dmg as f64 * protection).round() as i64;
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
        // flat cost: 9
        assert_eq!(
            old_vigor_hero - 9,
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
        // multiplicative power formula; 10% of effective damage passes through on block
        let hero_total_pow = gm.pm.current_player.stats.get_power_stat(false);
        let boss_total_armor = old_boss.stats.get_armor_stat(false);
        let boss_total_pow = old_boss.stats.get_power_stat(false);
        let power_factor = 1.0 + hero_total_pow as f64 / AttackType::POWER_SCALE;
        let raw_dmg = (35_f64 * power_factor).round() as i64;
        let defense = boss_total_armor as f64 + boss_total_pow as f64 / AttackType::DEFENSE_DIVISOR;
        let protection = AttackType::ARMOR_FACTOR / (AttackType::ARMOR_FACTOR + defense);
        let effective = (raw_dmg as f64 * protection).round() as i64;
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
        // flat cost: 9
        assert_eq!(
            old_vigor_hero - 9,
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
        // flat cost: 10 (see mana_cost of the atk)
        assert_eq!(
            old_mana_launcher - 10,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[MANA]
                .current
        );
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
        // flat cost: 18
        assert_eq!(
            old_mana_launcher - 18,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[MANA]
                .current
        );
        // "Magic power"
        // "ChangeMaxStat" 15
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
        // "ChangeMaxStat" 15
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
        let result = gm.launch_attack(Some("ChangeCurrentStatByValue-berseck"));
        let new_berserk = gm
            .pm
            .get_mut_active_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[BERSERK]
            .current;
        assert_eq!(result.new_game_atk_effects.len(), 1); // target himself
        // flat cost: 5, effect value +20
        assert_eq!(new_berserk, old_berserk_current - 5 + 20);
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
        let ra = gm.launch_attack(Some("Charge"));
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
            assert!(
                old_hp_boss
                    > gm.pm
                        .get_active_boss_character("Angmar_#1")
                        .unwrap()
                        .stats
                        .all_stats[HP]
                        .current,
                "non-dodged Charge must deal at least 1 damage"
            );
        }
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(2, gm.game_state.current_round);
        // remaining lotr heroes (Elara, Azrak, Thalia) — Charge exists, target not set so no damage
        assert_eq!(0, gm.process_nb_bosses_atk_in_a_row());
        let _ra = gm.launch_attack(Some("Charge"));
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(3, gm.game_state.current_round);
        let _ra = gm.launch_attack(Some("Charge"));
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(4, gm.game_state.current_round);
        let _ra = gm.launch_attack(Some("Charge"));
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
            if gm.is_round_auto() {
                // boss round: set a living hero as target so the individual attack lands
                if let Some(h) = gm
                    .pm
                    .active_heroes
                    .iter_mut()
                    .find(|h| h.stats.is_dead() != Some(true))
                {
                    h.character_rounds_info.is_current_target = true;
                }
                let _ = gm.process_nb_bosses_atk_in_a_row();
                let _ = gm.launch_attack(None);
            } else {
                // hero round: set a living boss as target so Charge lands
                if let Some(b) = gm
                    .pm
                    .active_bosses
                    .iter_mut()
                    .find(|b| b.stats.is_dead() != Some(true))
                {
                    b.character_rounds_info.is_current_target = true;
                }
                let _ = gm.launch_attack(Some("Charge"));
            }
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
}
