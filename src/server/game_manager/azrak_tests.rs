use super::*;

use crate::testing::testing_all_characters::{self};

/// Récupération Mordorienne: for each enemy killed last turn, Azrak gains
/// +10% Physical power (2 turns) and recovers 10% HP, Vigor, and Mana instantly.
/// This test uses 2 kills, so all bonuses are doubled.
#[test]
fn unit_azrak_recuperation_mordorienne_2_kills() {
    let mut gm = testing_all_characters::dxrpg_game_manager();
    gm.start_game();

    // Advance until Azrak is current player.
    let mut max_rounds = 30;
    while !gm.pm.current_player.id_name.contains("Azrak") && max_rounds > 0 {
        gm.new_round();
        max_rounds -= 1;
    }
    if !gm.pm.current_player.id_name.contains("Azrak") {
        return;
    }
    let azrak_id = gm.pm.current_player.id_name.clone();

    // Simulate 2 enemy kills on the previous turn.
    let prev_turn = gm.game_state.current_turn_nb.saturating_sub(1);
    gm.game_state
        .died_ennemies
        .insert(prev_turn, vec!["Boss_A".to_owned(), "Boss_B".to_owned()]);

    // Snapshot max stats and Physical power base (raw + equip, no buf_effect) before the attack.
    let hp_max = gm.pm.current_player.stats.all_stats[HP].max;
    let vigor_max = gm.pm.current_player.stats.all_stats[VIGOR].max;
    let mana_max = gm.pm.current_player.stats.all_stats[MANA].max;
    let phy_pow_max_before = gm.pm.current_player.stats.all_stats[PHYSICAL_POWER].max;
    // base_value = raw + equip (without existing buf_effect_percent, which the % is applied on top of)
    let phy_pow_attr = &gm.pm.current_player.stats.all_stats[PHYSICAL_POWER];
    let phy_base = phy_pow_attr.max_raw as i64
        + phy_pow_attr.buf_equip_value
        + phy_pow_attr.buf_equip_percent * phy_pow_attr.max_raw as i64 / 100;

    // Drain resources to ~50% so recovery is clearly visible and stays below cap.
    gm.pm.current_player.stats.all_stats[HP].current = hp_max / 2;
    gm.pm.current_player.stats.all_stats[VIGOR].current = vigor_max / 2;
    gm.pm.current_player.stats.all_stats[MANA].current = mana_max / 2;
    let hp_before = gm.pm.current_player.stats.all_stats[HP].current;
    let vigor_before = gm.pm.current_player.stats.all_stats[VIGOR].current;
    let mana_before = gm.pm.current_player.stats.all_stats[MANA].current;

    // Disable crits for determinism.
    gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;

    gm.launch_attack(Some("Récupération Mordorienne"));

    let azrak = gm.pm.get_active_hero_character(&azrak_id).unwrap();

    // 2 kills × 10% = +20% Physical power max for 2 turns.
    // ChangeMaxStat applies to base_value (raw + equip), not current max.
    let phy_pow_max_after = azrak.stats.all_stats[PHYSICAL_POWER].max;
    let expected_pow_max = phy_pow_max_before + (phy_base * 20 / 100) as u64;
    assert_eq!(
        expected_pow_max, phy_pow_max_after,
        "Physical power max must rise by 20% of base (2 kills × 10%)"
    );

    // 2 kills × 10% = instant +20% of max for each resource.
    // ChangeCurrentStat: full_amount = max * value / 100
    let expected_hp_gain = hp_max * 20 / 100;
    assert_eq!(
        hp_before + expected_hp_gain,
        azrak.stats.all_stats[HP].current,
        "HP must recover by 20% of max HP"
    );

    let expected_vigor_gain = vigor_max * 20 / 100;
    assert_eq!(
        vigor_before + expected_vigor_gain,
        azrak.stats.all_stats[VIGOR].current,
        "Vigor must recover by 20% of max Vigor"
    );

    let expected_mana_gain = mana_max * 20 / 100;
    assert_eq!(
        mana_before + expected_mana_gain,
        azrak.stats.all_stats[MANA].current,
        "Mana must recover by 20% of max Mana"
    );
}
