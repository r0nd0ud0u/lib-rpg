use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

use crate::{
    attack_type::AttackType,
    buffers::{BufTypes, Buffers},
    common::{
        attak_const::COEFF_CRIT_STATS, character_const::NB_TURN_SUM_AGGRO, effect_const::*,
        stats_const::*,
    },
    effect::{is_boosted_by_crit, is_effet_hot_or_dot, EffectOutcome, EffectParam},
    equipment::Equipment,
    game_state::GameState,
    powers::Powers,
    stats::Stats,
    utils,
};

/// ExtendedCharacter
#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct ExtendedCharacter {
    /// Fight information: Is the random character targeted by the current attack of other character
    #[serde(default, rename = "is_random_target")]
    pub is_random_target: bool,
    /// Fight information: TODO is_heal_atk_blocked
    #[serde(default, rename = "is_heal_atk_blocked")]
    pub is_heal_atk_blocked: bool,
    /// Fight information: Playing the first round of that tour
    #[serde(default, rename = "is_first_round")]
    pub is_first_round: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, Hash, PartialEq)]
pub enum AmountType {
    DamageRx = 0,
    DamageTx,
    HealRx,
    HealTx,
    OverHealRx,
    Aggro,
    EnumSize,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Character {
    /// Full Name of the character
    #[serde(rename = "Name")]
    pub name: String,
    /// Short name of the character
    #[serde(rename = "Short name")]
    pub short_name: String,
    /// Name of the photo of the character without extension
    #[serde(rename = "Photo")]
    pub photo_name: String,
    /// Stats about all the capacities and current state
    #[serde(rename = "Stats")]
    pub stats: Stats,
    /// Type of the character {Hero, Boss}
    #[serde(rename = "Type")]
    pub kind: CharacterType,
    /// Class of the character {Standard, Tank ...}
    #[serde(rename = "Class")]
    pub class: Class,
    /// Level of the character, start 1
    #[serde(rename = "Level")]
    pub level: u64,
    /// Experience of the character, start 0
    #[serde(rename = "Experience")]
    pub exp: u64,
    /// Experience to acquire to upgrade to next level
    pub next_exp_level: u64,
    /// key: body, value: equipmentName
    pub equipment_on: HashMap<String, Equipment>,
    /// key: attak name, value: AttakType struct
    pub attacks_list: HashMap<String, AttackType>,
    /// That vector contains all the atks from m_AttakList and is sorted by level.
    pub attacks_by_lvl: Vec<AttackType>,
    /// Main color theme of the character
    #[serde(rename = "Color")]
    pub color_theme: String,
    /// Fight information: last attack was critical
    pub is_last_atk_crit: bool,
    /// Fight information: damages transmitted or received through the fight
    #[serde(rename = "Tx-rx")]
    pub tx_rx: Vec<HashMap<u64, u64>>,
    /// Fight information: Enabled buf/debuf acquired through the fight
    #[serde(rename = "Buf-debuf")]
    pub all_buffers: Vec<Buffers>,
    /// Powers
    #[serde(rename = "Powers")]
    pub power: Powers,
    /// ExtendedCharacter
    #[serde(rename = "ExtendedCharacter")]
    pub extended_character: ExtendedCharacter,
    /// Fight information: attack can be blocked
    #[serde(rename = "is-blocking-atk")]
    pub is_blocking_atk: bool,
    /// Fight information: nb-actions-in-round
    #[serde(rename = "nb-actions-in-round")]
    pub actions_done_in_round: u64,
    /// Fight information: max-actions-in-round
    #[serde(rename = "max-actions-by-round")]
    pub max_actions_by_round: u64,
    /// TODO rank
    #[serde(rename = "Rank")]
    pub rank: u64,
    /// TODO shape
    #[serde(rename = "Shape")]
    pub shape: String,
}

impl Default for Character {
    fn default() -> Self {
        Character {
            name: String::from("default"),
            short_name: String::from("default"),
            photo_name: String::from("default"),
            stats: Stats::default(),
            kind: CharacterType::Hero,
            equipment_on: HashMap::new(),
            attacks_list: HashMap::new(),
            level: 1,
            exp: 0,
            next_exp_level: 100,
            attacks_by_lvl: vec![],
            color_theme: "dark".to_owned(),
            is_last_atk_crit: false,
            all_buffers: vec![],
            is_blocking_atk: false,
            power: Powers::default(),
            extended_character: ExtendedCharacter::default(),
            actions_done_in_round: 0,
            max_actions_by_round: 0,
            class: Class::Standard,
            tx_rx: vec![HashMap::new()],
            rank: 0,
            shape: String::new(),
        }
    }
}

/// Defines the type of player: hero -> player, boss -> computer.
/// "PascalCase" ensures that "Hero" and "Boss" from JSON map correctly to the Rust enum variants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum CharacterType {
    Hero,
    Boss,
}

/// Defines the class of the character
/// In the future, bonus and stats will be acquired.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Class {
    Standard,
    Tank,
}

impl Character {
    pub fn try_new_from_json<P: AsRef<Path>>(path: P) -> Result<Character> {
        if let Ok(mut value) = utils::read_from_json::<_, Character>(&path) {
            value.stats.init();
            Ok(value)
        } else {
            Err(anyhow!("Unknown file: {:?}", path.as_ref()))
        }
    }

    pub fn is_dead(&self) -> Option<bool> {
        if self.stats.all_stats.contains_key(HP) {
            Some(self.stats.all_stats[HP].current == 0)
        } else {
            None
        }
    }

    /**
     * @brief Character::InitAggroOnTurn
     * Set the aggro of m_LastTxRx to 0 on each turn
     * Assess the amount of aggro of the last 5 turns
     */
    pub fn init_aggro_on_turn(&mut self, turn_nb: usize) {
        if self.tx_rx.len() <= AmountType::Aggro as usize {
            return;
        }
        if let Some(aggro_stat) = self.stats.all_stats.get_mut(AGGRO) {
            aggro_stat.current = 0;
            for i in 1..NB_TURN_SUM_AGGRO + 1 {
                if i <= self.tx_rx[AmountType::Aggro as usize].len() {
                    let index = turn_nb - i;
                    aggro_stat.current += self.tx_rx[AmountType::Aggro as usize][&(index as u64)];
                }
            }
        }

        self.tx_rx[AmountType::Aggro as usize].insert(turn_nb as u64, 0);
    }

    /*
     * @brief Character::SetStatsOnEffect
     * @param stat
     * stat.m_RawMaxValue of a stat cannot be equal to 0.
     *
     * @param value
     * @param isPercent
     * @param updateEffect: false -> enable to update current value et max value
     * only with equipments buf.
     */
    pub fn set_stats_on_effect(
        &mut self,
        attribute_name: &str,
        value: i64,
        is_percent: bool,
        update_effect: bool,
    ) {
        let stat = self
            .stats
            .all_stats
            .get_mut(attribute_name)
            .expect("Stat not found");
        let ratio = utils::calc_ratio(stat.current as i64, stat.max as i64);
        if stat.max_raw == 0 {
            return;
        }
        let base_value =
            stat.max_raw + stat.buf_equip_value + stat.buf_equip_percent * stat.max_raw / 100;
        stat.max = base_value;
        if update_effect {
            if is_percent {
                if value > 0 {
                    stat.buf_effect_percent += value as u64;
                } else {
                    stat.buf_effect_percent = stat.buf_effect_percent.saturating_sub(value as u64);
                }
            } else if value > 0 {
                stat.buf_effect_value += value as u64;
            } else {
                stat.buf_effect_value = stat.buf_effect_percent.saturating_sub(value as u64);
            }
        }
        stat.max = base_value + stat.buf_effect_value + stat.buf_effect_percent * base_value / 100;
        stat.current = (stat.max as f64 * ratio).round() as u64;
    }

    // TODO if i remove a malus percent for DamageTx with EFFECT_CHANGE_MAX_DAMAGES_BY_PERCENT, how can if make the difference with a value which is not percent
    pub fn remove_malus_effect(&mut self, ep: &EffectParam) {
        if ep.effect_type == EFFECT_IMPROVE_BY_PERCENT_CHANGE {
            self.set_stats_on_effect(&ep.stats_name, -ep.value, true, true);
        }
        if ep.effect_type == EFFECT_IMPROVEMENT_STAT_BY_VALUE {
            self.set_stats_on_effect(&ep.stats_name, -ep.value, false, true);
        }
        if ep.effect_type == EFFECT_BLOCK_HEAL_ATK {
            self.extended_character.is_heal_atk_blocked = false;
        }
        if ep.effect_type == EFFECT_CHANGE_MAX_DAMAGES_BY_PERCENT {
            self.update_buf(BufTypes::DamageTx, -ep.value, true, "");
        }
        if ep.effect_type == EFFECT_CHANGE_DAMAGES_RX_BY_PERCENT {
            self.update_buf(BufTypes::DamageRx, -ep.value, true, "");
        }
        if ep.effect_type == EFFECT_CHANGE_HEAL_RX_BY_PERCENT {
            self.update_buf(BufTypes::HealRx, -ep.value, true, "");
        }
        if ep.effect_type == EFFECT_CHANGE_HEAL_TX_BY_PERCENT {
            self.update_buf(BufTypes::HealTx, -ep.value, true, "");
        }
    }

    pub fn update_buf(&mut self, buf_type: BufTypes, value: i64, is_percent: bool, stat: &str) {
        if let Some(buf) = self.all_buffers.get_mut(buf_type as usize) {
            buf.value += value;
            buf.is_percent = is_percent;
            buf.all_stats_name.push(stat.to_string());
        }
    }

    pub fn process_one_effect(
        &mut self,
        target: &Character,
        ep: &EffectParam,
        _from_launch: bool,
        atk: &AttackType,
        game_state: &GameState,
        is_crit: bool,
    ) -> (EffectParam, String) {
        let mut output: EffectParam = EffectParam::default();
        let mut result: String = String::new();

        // Preprocess effectParam before applying it
        // update effectParam
        if is_crit && is_boosted_by_crit(&ep.effect_type) {
            output.sub_value_effect = (COEFF_CRIT_STATS * ep.sub_value_effect as f64) as i64;
            output.value = (COEFF_CRIT_STATS * ep.value as f64) as i64;
        }
        // conditions
        if ep.effect_type == CONDITION_ENNEMIES_DIED {
            output.value += game_state.died_ennemies[&(game_state.current_turn_nb - 1)].len()
                as i64
                * output.sub_value_effect;
            output.effect_type = EFFECT_IMPROVE_BY_PERCENT_CHANGE.to_owned();
        }

        // Process effect param
        let (effect_log, _new_effect_param) = self.process_effect_type(ep, target, atk);
        result += &effect_log;

        (output, result)
    }

    // TODO do not change ep but update output
    pub fn apply_one_effect(
        &mut self,
        target: &Character,
        ep: &EffectParam,
        from_launch: bool,
        atk: &AttackType,
        reload: bool,
        is_crit: bool,
    ) -> EffectOutcome {
        let mut output = EffectOutcome::default();
        let mut result = String::new();

        // init effect param
        output.new_effect_param = ep.clone();

        let (_max_amount_sent, real_amount_sent) =
            Self::process_current_value_on_effect(ep, &self.stats, from_launch, target, is_crit);

        if !reload {
            result += &Self::regen_into_damage(real_amount_sent, &ep.stats_name);
            let buf = self.all_buffers.get(BufTypes::ChangeByHealValue as usize);
            if real_amount_sent > 0 && buf.is_some() && buf.unwrap().is_passive_enabled {
                let stats = buf.unwrap().all_stats_name.clone();
                for stat in stats {
                    let mut ep = EffectParam {
                        effect_type: EFFECT_IMPROVEMENT_STAT_BY_VALUE.to_string(),
                        value: real_amount_sent,
                        is_magic_atk: true,
                        stats_name: stat.clone(),
                        nb_turns: buf.unwrap().value,
                        ..Default::default()
                    };
                    ep.effect_type = EFFECT_IMPROVEMENT_STAT_BY_VALUE.to_string();
                    ep.value = real_amount_sent;
                    ep.is_magic_atk = true;
                    ep.stats_name = stat;
                    ep.nb_turns = buf.unwrap().value;
                    let (effect_log, new_effect_param) = self.process_effect_type(&ep, target, atk);
                    result += &effect_log;
                    // TODO add log
                    // result += target->ProcessOutputLogOnEffect(ep, ep.value, fromLaunch, 1,atk.name, ep.value);
                    output.new_effects.push(new_effect_param);
                }
            }
        }
        if ep.stats_name == HP && is_effet_hot_or_dot(&ep.effect_type) {
            // process effect and return it in EffectOutcome
            // ep.value = max_amount_sent;
            // TODO
        }
        output
    }

    // TODO
    pub fn process_effect_type(
        &self,
        ep: &EffectParam,
        target: &Character,
        atk: &AttackType,
    ) -> (String, EffectParam) {
        if target.is_dead().unwrap_or(false) {
            return (String::new(), ep.clone());
        }
        let mut output_log: String = String::new();
        let mut new_effect_param = ep.clone();
        new_effect_param.number_of_applies = 1;

        match ep.effect_type.as_str() {
            EFFECT_NB_COOL_DOWN => {
                if self.name == target.name {
                    output_log =
                        format!("Cooldown actif sur {} de {} tours.", atk.name, ep.nb_turns);
                }
                // example cooldown for 2 turns
                // T1 no change
                // T2 cooldown, nb turn -1
                // T3 cooldown, nb turn -1
                // T4 nb turn -1 => effect finished => can be launched
                // => for a cooldown of n=2 turns, the init value of nbTurns = n + 1;
                new_effect_param.nb_turns += 1;
            }
            EFFECT_NB_DECREASE_ON_TURN => {
                // TODO
                new_effect_param.number_of_applies = 0; // TODO
            }
            _ => {}
        }
        // Must be filled before changing value of nbTurns
        if ep.effect_type == EFFECT_REINIT {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_DELETE_BAD {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_IMPROVE_HOTS {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_BOOSTED_BY_HOTS {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_CHANGE_MAX_DAMAGES_BY_PERCENT {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_CHANGE_DAMAGES_RX_BY_PERCENT {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_CHANGE_HEAL_RX_BY_PERCENT {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_CHANGE_HEAL_TX_BY_PERCENT {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_IMPROVE_BY_PERCENT_CHANGE {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_IMPROVEMENT_STAT_BY_VALUE {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_REPEAT_AS_MANY_AS {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_INTO_DAMAGE {
            // TODO
            return (String::new(), ep.clone());
        }
        (output_log, new_effect_param)
    }

    pub fn process_current_value_on_effect(
        _ep: &EffectParam,
        _stats: &Stats,
        _from_launch: bool,
        _target: &Character,
        _is_crit: bool,
    ) -> (i64, i64) {
        (0, 0)
    }
    pub fn regen_into_damage(_real_amount_sent: i64, _stats_name: &str) -> String {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        buffers::BufTypes,
        character::{CharacterType, Class},
        common::{effect_const::*, stats_const::*},
        effect::EffectParam,
    };

    use super::Character;

    #[test]
    fn unit_try_new_from_json() {
        let file_path = "./tests/characters/test.json"; // Path to the JSON file
        let c = Character::try_new_from_json(file_path);
        assert!(c.is_ok());
        let c = c.unwrap();
        // name
        assert_eq!("Super test", c.name);
        assert_eq!("Test", c.short_name);
        // buf-debuf
        assert_eq!(12, c.all_buffers.len());
        // TODO change
        //assert_eq!("hp,mana", c.all_buffers[0].all_stats_name);
        assert_eq!(3, c.all_buffers[0].buf_type);
        assert_eq!(false, c.all_buffers[0].is_passive_enabled);
        assert_eq!(true, c.all_buffers[0].is_percent);
        assert_eq!(100, c.all_buffers[0].value);
        // Class
        assert_eq!(Class::Standard, c.class);
        // Color
        assert_eq!("green", c.color_theme);
        // Experience
        assert_eq!(50, c.exp);
        // extended character
        assert_eq!(false, c.extended_character.is_first_round);
        assert_eq!(true, c.extended_character.is_heal_atk_blocked);
        assert_eq!(false, c.extended_character.is_random_target);
        // level
        assert_eq!(1, c.level);
        // photo
        assert_eq!("phototest", c.photo_name);
        // powers
        assert_eq!(false, c.power.is_crit_heal_after_crit);
        assert_eq!(true, c.power.is_damage_tx_heal_needy_ally);
        // rank
        assert_eq!(4, c.rank);
        // shape
        assert_eq!("", c.shape);
        // stats
        // stats - aggro
        assert_eq!(0, c.stats.all_stats[AGGRO].current);
        assert_eq!(9999, c.stats.all_stats[AGGRO].max);
        // stats - aggro rate
        assert_eq!(1, c.stats.all_stats[AGGRO_RATE].current);
        assert_eq!(1, c.stats.all_stats[AGGRO_RATE].max);
        // stats - berseck
        assert_eq!(200, c.stats.all_stats[BERSECK].current);
        assert_eq!(200, c.stats.all_stats[BERSECK].max);
        // stats - berseck_rate
        assert_eq!(1, c.stats.all_stats[BERSECK_RATE].current);
        assert_eq!(1, c.stats.all_stats[BERSECK_RATE].max);
        // stats - critical_strike
        assert_eq!(10, c.stats.all_stats[CRITICAL_STRIKE].current);
        assert_eq!(10, c.stats.all_stats[CRITICAL_STRIKE].max);
        // stats - dodge
        assert_eq!(5, c.stats.all_stats[DODGE].current);
        assert_eq!(5, c.stats.all_stats[DODGE].max);
        // stats - hp
        assert_eq!(1, c.stats.all_stats[HP].current);
        assert_eq!(135, c.stats.all_stats[HP].max);
        assert_eq!(135, c.stats.all_stats[HP].max_raw);
        assert_eq!(1, c.stats.all_stats[HP].current_raw);
        // stats - hp_regeneration
        assert_eq!(7, c.stats.all_stats[HP_REGEN].current);
        assert_eq!(7, c.stats.all_stats[HP_REGEN].max);
        // stats - magic_armor
        assert_eq!(10, c.stats.all_stats[MAGICAL_ARMOR].current);
        assert_eq!(10, c.stats.all_stats[MAGICAL_ARMOR].max);
        // stats - magic_power
        assert_eq!(20, c.stats.all_stats[MAGICAL_POWER].current);
        assert_eq!(20, c.stats.all_stats[MAGICAL_POWER].max);
        // stats - mana
        assert_eq!(200, c.stats.all_stats[MANA].current);
        assert_eq!(200, c.stats.all_stats[MANA].max);
        // stats - mana_regeneration
        assert_eq!(7, c.stats.all_stats[MANA_REGEN].current);
        assert_eq!(7, c.stats.all_stats[MANA_REGEN].max);
        // stats - physical_armor
        assert_eq!(5, c.stats.all_stats[PHYSICAL_ARMOR].current);
        assert_eq!(5, c.stats.all_stats[PHYSICAL_ARMOR].max);
        // stats - physical_power
        assert_eq!(10, c.stats.all_stats[PHYSICAL_POWER].current);
        assert_eq!(10, c.stats.all_stats[PHYSICAL_POWER].max);
        // stats - speed
        assert_eq!(212, c.stats.all_stats[SPEED].current);
        assert_eq!(212, c.stats.all_stats[SPEED].max);
        // stats - speed_regeneration
        assert_eq!(12, c.stats.all_stats[SPEED_REGEN].current);
        assert_eq!(12, c.stats.all_stats[SPEED_REGEN].max);
        // stats - vigor
        assert_eq!(200, c.stats.all_stats[VIGOR].current);
        assert_eq!(200, c.stats.all_stats[VIGOR].max);
        // stats - vigor_regeneration
        assert_eq!(5, c.stats.all_stats[VIGOR_REGEN].current);
        assert_eq!(5, c.stats.all_stats[VIGOR_REGEN].max);
        // tx-rx
        assert_eq!(6, c.tx_rx.len());
        /*         assert_eq!(0, c.tx_rx[2].tx_rx_size);
        assert_eq!(2, c.tx_rx[2].tx_rx_type); */
        // Type - kind
        assert_eq!(CharacterType::Hero, c.kind);
        // is-blocking-atk
        assert_eq!(false, c.is_blocking_atk);
        // max_actions_by_round
        assert_eq!(1, c.max_actions_by_round);
        // nb-actions-in-round
        assert_eq!(0, c.actions_done_in_round);

        let file_path = "./tests/characters/wrong.json";
        assert!(Character::try_new_from_json(file_path).is_err());
    }

    #[test]
    fn unit_is_dead() {
        let mut c = Character::default();
        c.stats.init();
        assert!(c.is_dead().is_some());
        assert_eq!(true, c.is_dead().unwrap());
        c.stats.all_stats.get_mut(HP).unwrap().current = 15;
        assert_eq!(false, c.is_dead().unwrap());
    }

    #[test]
    fn unit_init_aggro_on_turn() {
        let mut c = Character::default();
        c.stats.init();
        c.init_aggro_on_turn(1);
        assert_eq!(0, c.stats.all_stats[AGGRO].current);
        c.tx_rx.push(HashMap::new());
        c.tx_rx.push(HashMap::new());
        c.tx_rx.push(HashMap::new());
        c.tx_rx.push(HashMap::new());
        c.tx_rx.push(HashMap::new());
        c.tx_rx.push(HashMap::new());
        c.tx_rx[5].insert(1, 10);
        c.init_aggro_on_turn(2);
        assert_eq!(10, c.stats.all_stats[AGGRO].current);
        c.tx_rx[5].insert(2, 20);
        c.init_aggro_on_turn(3);
        assert_eq!(30, c.stats.all_stats[AGGRO].current);
        c.tx_rx[5].insert(3, 30);
        c.init_aggro_on_turn(4);
        assert_eq!(60, c.stats.all_stats[AGGRO].current);
        c.tx_rx[5].insert(4, 40);
        c.init_aggro_on_turn(5);
        assert_eq!(100, c.stats.all_stats[AGGRO].current);
        c.tx_rx[5].insert(5, 50);
        c.init_aggro_on_turn(6);
        assert_eq!(150, c.stats.all_stats[AGGRO].current);
        c.tx_rx[5].insert(6, 60);
        c.init_aggro_on_turn(7);
        assert_eq!(200, c.stats.all_stats[AGGRO].current);
        c.tx_rx[5].insert(7, 70);
        c.init_aggro_on_turn(8);
        assert_eq!(250, c.stats.all_stats[AGGRO].current);
        c.tx_rx[5].insert(8, 80);
        c.init_aggro_on_turn(9);
        assert_eq!(300, c.stats.all_stats[AGGRO].current);
        c.tx_rx[5].insert(9, 90);
        c.init_aggro_on_turn(10);
        assert_eq!(350, c.stats.all_stats[AGGRO].current);
    }

    #[test]
    fn unit_set_stats_on_effect() {
        let file_path = "./tests/characters/test.json"; // Path to the JSON file
        let c = Character::try_new_from_json(file_path);
        assert!(c.is_ok());
        let mut c = c.unwrap();
        c.set_stats_on_effect(HP, 10, false, true);
        assert_eq!(145, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        c.set_stats_on_effect(HP, -10, false, true);
        assert_eq!(135, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        c.set_stats_on_effect(HP, 10, true, true);
        assert_eq!(148, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        c.set_stats_on_effect(HP, -10, true, true);
        assert_eq!(135, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
    }

    #[test]
    fn unitremove_malus_effect() {
        let file_path = "./tests/characters/test.json"; // Path to the JSON file
        let c = Character::try_new_from_json(file_path);
        assert!(c.is_ok());
        let mut c = c.unwrap();
        let ep = EffectParam {
            effect_type: EFFECT_IMPROVE_BY_PERCENT_CHANGE.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };
        c.remove_malus_effect(&ep);
        assert_eq!(135, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        let ep = EffectParam {
            effect_type: EFFECT_IMPROVEMENT_STAT_BY_VALUE.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };
        c.remove_malus_effect(&ep);
        assert_eq!(135, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        let ep = EffectParam {
            effect_type: EFFECT_BLOCK_HEAL_ATK.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };
        c.remove_malus_effect(&ep);
        assert_eq!(false, c.extended_character.is_heal_atk_blocked);
        let ep = EffectParam {
            effect_type: EFFECT_CHANGE_MAX_DAMAGES_BY_PERCENT.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };
        c.remove_malus_effect(&ep);
        assert_eq!(-10, c.all_buffers[BufTypes::DamageTx as usize].value);
        let ep = EffectParam {
            effect_type: EFFECT_CHANGE_DAMAGES_RX_BY_PERCENT.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };
        c.remove_malus_effect(&ep);
        assert_eq!(-10, c.all_buffers[BufTypes::DamageRx as usize].value);
        let ep = EffectParam {
            effect_type: EFFECT_CHANGE_HEAL_RX_BY_PERCENT.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };
        c.remove_malus_effect(&ep);
        assert_eq!(-10, c.all_buffers[BufTypes::HealRx as usize].value);
        let ep = EffectParam {
            effect_type: EFFECT_CHANGE_HEAL_TX_BY_PERCENT.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };
        c.remove_malus_effect(&ep);
        assert_eq!(-10, c.all_buffers[BufTypes::HealTx as usize].value);
    }
}
