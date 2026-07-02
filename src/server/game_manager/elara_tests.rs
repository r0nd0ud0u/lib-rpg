use super::*;

use crate::character_mod::attack_type::AttackType;
use crate::character_mod::buffers::BufKinds;
use crate::testing::testing_all_characters::{self};

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

    let result = gm.launch_attack(Some("Frappe élémentaire"));

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
    let expected_heal = (actual_damage as u64 * 25 / 100).min(azrak_hp_max - 10);
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

    // Expected magic damage: rebalanced base value=76, Elara mag power, boss mag armor+pow
    let hero_mag_pow = gm.pm.current_player.stats.get_power_stat(true);
    let boss_mag_armor = gm.pm.active_bosses[0].stats.get_armor_stat(true);
    let boss_mag_pow = gm.pm.active_bosses[0].stats.get_power_stat(true);
    let power_factor = 1.0 + hero_mag_pow as f64 / AttackType::POWER_SCALE;
    let raw_dmg = (76_f64 * power_factor).round() as i64;
    let defense = boss_mag_armor as f64 + boss_mag_pow as f64 / AttackType::DEFENSE_DIVISOR;
    let protection = AttackType::ARMOR_FACTOR / (AttackType::ARMOR_FACTOR + defense);
    let expected_dmg = (raw_dmg as f64 * protection).round() as i64;

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
    gm.pm.current_player.character_rounds_info.tx_rx[AmountType::DamageTx as usize].insert(1, 200);
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
    let expected_mana = (old_mana as i64 - (15 * mana_max / 100) as i64 + mana_regen as i64) as u64;
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
    let mana_max = gm.pm.current_player.stats.all_stats[MANA].max;
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

    // Mana cost: 24% of max mana
    assert_eq!(
        old_mana - 24 * mana_max / 100,
        gm.pm
            .get_active_hero_character(elara_id)
            .unwrap()
            .stats
            .all_stats[MANA]
            .current,
        "Non sans raison costs 24% of max mana"
    );
}
