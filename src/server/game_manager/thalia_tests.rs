use super::*;

use crate::character_mod::buffers::BufKinds;
use crate::testing::testing_all_characters::testing_test_ally1_vs_test_boss1;

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
    // The ChangeMaxStat effect shares the ApplyEffectInit count
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
                    kind: BufKinds::ChangeCurrentStat,
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
        gae.processed_effect_param.input_effect_param.buffer.kind == BufKinds::CooldownTurnsNumber
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
                kind: BufKinds::ChangeCurrentStat,
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
                kind: BufKinds::ChangeCurrentStat,
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
            gae.processed_effect_param.input_effect_param.buffer.kind == BufKinds::ChangeCurrentStat
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

/// Sève Régénératrice targets a single ally (not necessarily the caster) and, unlike Eveil
/// de la forêt, applies no fresh HOT alongside ReinitBuf — so this test can't pass by
/// accident from a newly-created HOT masking whether the pre-existing one was actually reset.
/// It seeds the aged HOT only on the target (never on Thalia), so a pass here can only be
/// explained by ReinitBuf resetting the real target's own effects.
#[test]
fn unit_seve_regeneratrice_reinit_hot_counter_on_non_caster_ally() {
    use crate::character_mod::buffers::BufKinds;

    let (mut gm, thalia_id) = setup_thalia_turn();
    if !thalia_id.contains("Thalia") {
        return;
    }

    let Some(target_id) = gm
        .pm
        .active_heroes
        .iter()
        .find(|h| h.id_name != thalia_id)
        .map(|h| h.id_name.clone())
    else {
        return;
    };

    let aged_hot_ep = crate::character_mod::effect::ProcessedEffectParam {
        input_effect_param: crate::character_mod::effect::EffectParam {
            nb_turns: 4,
            buffer: crate::character_mod::buffers::Buffer {
                kind: BufKinds::ChangeCurrentStat,
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
    if let Some(target) = gm
        .pm
        .active_heroes
        .iter_mut()
        .find(|h| h.id_name == target_id)
    {
        target.character_rounds_info.all_effects.push(aged_hot_gae);
    }

    gm.pm
        .set_targeted_characters(&thalia_id, "Sève Régénératrice");
    gm.launch_attack(Some("Sève Régénératrice"));

    let target = gm
        .pm
        .active_heroes
        .iter()
        .find(|h| h.id_name == target_id)
        .unwrap();
    let reset = target.character_rounds_info.all_effects.iter().any(|gae| {
        gae.processed_effect_param.input_effect_param.buffer.kind == BufKinds::ChangeCurrentStat
            && gae
                .processed_effect_param
                .input_effect_param
                .buffer
                .stats_name
                == HP
            && gae.processed_effect_param.input_effect_param.nb_turns == 4
            && gae.processed_effect_param.counter_turn == 0
    });
    assert!(
        reset,
        "Sève Régénératrice must reset the HP HOT counter on the actual target ({}), not just the caster",
        target_id
    );
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
                kind: BufKinds::ChangeCurrentStat,
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
