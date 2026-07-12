use super::*;

use crate::character_mod::attack_type::AttackType;
use crate::character_mod::buffers::BufKinds;
use crate::testing::testing_all_characters::{self, testing_test_ally1_vs_test_boss1};

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
                kind: BufKinds::ChangeCurrentStat,
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

/// Integration test: Thraïn's passive ChangeCurrentStat(Dodge, 10) raises his
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

    // Set berserk to 60; the attack costs a flat 20 rage per apply and fires as many
    // times as possible (RepeatAsManyAsPossible) until rage is exhausted.
    // nb_applies = floor(60 / actual_cost).
    gm.pm.current_player.stats.all_stats[BERSERK].current = 60;
    let berseck_cost = 20u64;
    let actual_cost = berseck_cost.max(1);
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
    // Force a deterministic non-crit roll: ChangeMaxStat is boosted by crit (×COEFF_CRIT_STATS),
    // so a random crit on this self-buff would inflate the +40 assertion below (flaky otherwise).
    gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;

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
            e.processed_effect_param.input_effect_param.buffer.kind == BufKinds::CooldownTurnsNumber
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
    let berseck_cost = 15u64;
    let cost_deducted = berseck_cost;

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
    let boss_phy_pows: Vec<i64> = gm
        .pm
        .active_bosses
        .iter()
        .map(|b| b.stats.get_power_stat(false))
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

    // All 3 bosses take physical damage with rebalanced base value 67
    for (i, ((old_hp, phy_armor), boss_phy_pow)) in old_boss_hps
        .iter()
        .zip(boss_phy_armors.iter())
        .zip(boss_phy_pows.iter())
        .enumerate()
    {
        let boss_id = gm.pm.active_bosses[i].id_name.clone();
        let new_hp = gm
            .pm
            .get_active_boss_character(&boss_id)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let power_factor = 1.0 + thrain_phy_pow as f64 / AttackType::POWER_SCALE;
        let raw_dmg = (67_f64 * power_factor).round() as i64;
        let defense = *phy_armor as f64 + *boss_phy_pow as f64 / AttackType::DEFENSE_DIVISOR;
        let protection = AttackType::ARMOR_FACTOR / (AttackType::ARMOR_FACTOR + defense);
        let expected_dmg = (raw_dmg as f64 * protection).round() as i64;
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
