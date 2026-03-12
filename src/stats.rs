use indexmap::IndexMap;
use std::{cmp::Ordering, collections::HashMap};

use serde::{Deserialize, Serialize};

use crate::{
    common::{
        character_const::NB_TURN_SUM_AGGRO,
        stats_const::{
            AGGRO, AGGRO_RATE, BERSECK_RATE, BERSERK, CRITICAL_STRIKE, DODGE, HP, HP_REGEN,
            MAGICAL_ARMOR, MAGICAL_POWER, MANA, MANA_REGEN, PHYSICAL_ARMOR, PHYSICAL_POWER, SPEED,
            SPEED_REGEN, VIGOR, VIGOR_REGEN,
        },
    },
    equipment::Equipment,
    utils,
};

/// Define allt the paramaters of tx-rx
#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct TxRx {
    /// TODO use?
    #[serde(rename = "Tx-rx-size")]
    pub tx_rx_size: u64,
}

/// Define all the parameter of an attribute of a stat
#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct Attribute {
    /// Current value of the stat, with equipment and buf/debuf included
    #[serde(rename = "Current")]
    pub current: u64,
    /// Current raw value of the stat, WITHOUT equipment and buf/debuf included
    pub current_raw: u64,
    /// Max value of the stat, with equipment and buf/debuf included
    #[serde(rename = "Max")]
    pub max: u64,
    /// Raw Max value of the stat, WITHOUT equipment and buf/debuf included
    pub max_raw: u64,
    /// All buffer values are added in one value
    pub buf_effect_value: i64,
    /// All buffer percentage are added in one percent value
    pub buf_effect_percent: i64,
    /// All buffer equipment are added in one value
    #[serde(rename = "equip_value")]
    pub buf_equip_value: i64,
    /// All buffer equipment are added in one value
    #[serde(rename = "equip_percent")]
    pub buf_equip_percent: i64,
}

impl Ord for Attribute {
    fn cmp(&self, other: &Self) -> Ordering {
        self.current.cmp(&other.current)
    }
}

impl PartialOrd for Attribute {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other)) // Ensures a total order
    }
}

impl Attribute {
    pub fn sync_raw_values(&mut self) {
        self.current_raw = self.current;
        self.max_raw = self.max;
    }
}

/// Define all the parameters of the stats of one character
#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct Stats {
    #[serde(rename = "Aggro")]
    aggro: Attribute,

    #[serde(rename = "Aggro rate")]
    aggro_rate: Attribute,

    #[serde(rename = "Magical armor")]
    magical_armor: Attribute,

    #[serde(rename = "Physical armor")]
    physical_armor: Attribute,

    #[serde(rename = "Magical power")]
    magic_power: Attribute,

    #[serde(rename = "Physical power")]
    physical_power: Attribute,

    #[serde(rename = "HP")]
    hp: Attribute,

    #[serde(rename = "Mana")]
    mana: Attribute,

    #[serde(rename = "Vigor")]
    vigor: Attribute,

    #[serde(rename = "Berseck")]
    berseck: Attribute,

    #[serde(rename = "Berseck rate")]
    berseck_rate: Attribute,

    #[serde(rename = "Speed")]
    speed: Attribute,

    #[serde(rename = "Critical strike")]
    critical_strike: Attribute,

    #[serde(rename = "Dodge")]
    dodge: Attribute,

    #[serde(rename = "HP regeneration")]
    hp_regeneration: Attribute,

    #[serde(rename = "Mana regeneration")]
    mana_regeneration: Attribute,

    #[serde(rename = "Vigor regeneration")]
    vigor_regeneration: Attribute,

    #[serde(rename = "Speed regeneration")]
    speed_regeneration: Attribute,

    pub all_stats: IndexMap<String, Attribute>,
}

impl Stats {
    pub fn init(&mut self) {
        self.aggro.sync_raw_values();
        self.aggro_rate.sync_raw_values();
        self.magical_armor.sync_raw_values();
        self.physical_armor.sync_raw_values();
        self.magic_power.sync_raw_values();
        self.physical_power.sync_raw_values();
        self.hp.sync_raw_values();
        self.mana.sync_raw_values();
        self.vigor.sync_raw_values();
        self.berseck.sync_raw_values();
        self.berseck_rate.sync_raw_values();
        self.speed.sync_raw_values();
        self.critical_strike.sync_raw_values();
        self.dodge.sync_raw_values();
        self.hp_regeneration.sync_raw_values();
        self.mana_regeneration.sync_raw_values();
        self.vigor_regeneration.sync_raw_values();
        self.speed_regeneration.sync_raw_values();

        self.all_stats.insert(AGGRO.to_string(), self.aggro.clone());
        self.all_stats
            .insert(AGGRO_RATE.to_string(), self.aggro_rate.clone());
        self.all_stats
            .insert(MAGICAL_ARMOR.to_string(), self.magical_armor.clone());
        self.all_stats
            .insert(PHYSICAL_ARMOR.to_string(), self.physical_armor.clone());
        self.all_stats
            .insert(MAGICAL_POWER.to_string(), self.magic_power.clone());
        self.all_stats
            .insert(PHYSICAL_POWER.to_string(), self.physical_power.clone());
        self.all_stats.insert(HP.to_string(), self.hp.clone());
        self.all_stats.insert(MANA.to_string(), self.mana.clone());
        self.all_stats.insert(VIGOR.to_string(), self.vigor.clone());
        self.all_stats
            .insert(BERSERK.to_string(), self.berseck.clone());
        self.all_stats
            .insert(BERSECK_RATE.to_string(), self.berseck_rate.clone());
        self.all_stats.insert(SPEED.to_string(), self.speed.clone());
        self.all_stats
            .insert(CRITICAL_STRIKE.to_string(), self.critical_strike.clone());
        self.all_stats.insert(DODGE.to_string(), self.dodge.clone());
        self.all_stats
            .insert(HP_REGEN.to_string(), self.hp_regeneration.clone());
        self.all_stats
            .insert(MANA_REGEN.to_string(), self.mana_regeneration.clone());
        self.all_stats
            .insert(VIGOR_REGEN.to_string(), self.vigor_regeneration.clone());
        self.all_stats
            .insert(SPEED_REGEN.to_string(), self.speed_regeneration.clone());
    }

    pub fn get_mut_value(&mut self, name: &str) -> &mut Attribute {
        self.all_stats
            .get_mut(name)
            .expect("key missing in all_stats")
    }

    pub fn is_energy_stat(name: &str) -> bool {
        name == HP || name == MANA || name == VIGOR || name == BERSERK
    }

    pub fn get_power_stat(&self, is_magic: bool) -> i64 {
        let pow = if is_magic {
            &self.all_stats[MAGICAL_POWER]
        } else {
            &self.all_stats[PHYSICAL_POWER]
        };
        pow.current as i64
    }
    pub fn get_armor_stat(&self, is_magic: bool) -> i64 {
        let armor = if is_magic {
            &self.all_stats[MAGICAL_ARMOR]
        } else {
            &self.all_stats[PHYSICAL_ARMOR]
        };
        armor.current as i64
    }

    pub fn is_dead(&self) -> Option<bool> {
        if self.all_stats.contains_key(HP) {
            Some(self.all_stats[HP].current == 0)
        } else {
            None
        }
    }

    pub fn modify_stat_current(&mut self, attribute_name: &str, delta: i64) {
        let stat = self
            .all_stats
            .get_mut(attribute_name)
            .unwrap_or_else(|| panic!("Stat not found: {}", attribute_name));

        let mut new_value = stat.current as i128 + delta as i128;

        // always prevent negative
        if new_value < 0 {
            new_value = 0;
        }

        // only clamp if max is defined
        if stat.max > 0 {
            new_value = new_value.min(stat.max as i128);
        }

        stat.current = new_value as u64;
    }

    /// stat.m_RawMaxValue of a stat cannot be equal to 0.
    /// updateEffect: false -> enable to update current value et max value only with equipments buf
    pub fn set_stats_on_effect(
        &mut self,
        attribute_name: &str,
        value: i64,
        is_percent: bool,
        update_effect: bool,
    ) {
        let stat = self
            .all_stats
            .get_mut(attribute_name)
            .unwrap_or_else(|| panic!("Stat not found: {}", attribute_name));
        if stat.max_raw == 0 {
            return;
        }
        if update_effect {
            if is_percent {
                stat.buf_effect_percent += value;
            } else {
                stat.buf_effect_value += value;
            }
        }
        let base_value = stat.max_raw as i64
            + stat.buf_equip_value
            + stat.buf_equip_percent * stat.max_raw as i64 / 100;
        let new_base =
            base_value + stat.buf_effect_value + stat.buf_effect_percent * base_value / 100;
        stat.max = new_base.max(0) as u64;
        // stats current
        let ratio = utils::calc_ratio(stat.current as i64, stat.max as i64);
        stat.current = (stat.max as f64 * ratio).round() as u64;
    }

    pub fn apply_equipment_on_stats(&mut self, equipment_on: &Vec<Equipment>) {
        for equipment in equipment_on {
            for (stat_name, stat_effect) in &equipment.stats.all_stats {
                if stat_effect.buf_equip_percent == 0 && stat_effect.buf_equip_value == 0 {
                    continue;
                }

                let attr = self.get_mut_value(stat_name);
                attr.buf_equip_value += stat_effect.buf_equip_value;
                attr.buf_equip_percent += stat_effect.buf_equip_percent;

                let ratio = utils::calc_ratio(attr.current as i64, attr.max as i64);
                attr.max = attr.max_raw
                    + attr.buf_equip_value as u64
                    + attr.max_raw * attr.buf_equip_percent as u64 / 100;

                attr.current = (attr.max as f64 * ratio).round() as u64;
            }
        }
    }

    pub fn init_aggro_on_turn(&mut self, turn_nb: usize, all_aggro: &HashMap<u64, i64>) {
        if let Some(aggro_stat) = self.all_stats.get_mut(AGGRO) {
            aggro_stat.current = 0;
            let mut index: i64;
            for i in 1..NB_TURN_SUM_AGGRO + 1 {
                index = turn_nb as i64 - i as i64;
                if index < 0 {
                    break;
                }
                if i <= all_aggro.len() {
                    let aggro = *all_aggro.get(&(index as u64)).unwrap_or(&0);
                    aggro_stat.current = aggro_stat.current.saturating_add(aggro as u64);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        character::Character, common::paths_const::TEST_OFFLINE_ROOT,
        testing_all_characters::testing_all_equipment,
    };

    use super::*;

    #[test]
    pub fn unit_stats() {
        let mut stats = Stats::default();
        stats.init();
        let mut stats2 = stats.clone();
        stats2.get_mut_value(HP).current = 10;
        assert_eq!(stats.get_mut_value(HP).current, 0);
        assert_eq!(stats2.get_mut_value(HP).current, 10);
    }

    #[test]
    pub fn unit_stats_get_power_stat() {
        let mut stats = Stats::default();
        stats.init();
        stats.get_mut_value(MAGICAL_POWER).current = 10;
        stats.get_mut_value(PHYSICAL_POWER).current = 20;
        assert_eq!(stats.get_power_stat(true), 10);
        assert_eq!(stats.get_power_stat(false), 20);
    }

    #[test]
    pub fn unit_stats_get_armor_stat() {
        let mut stats = Stats::default();
        stats.init();
        stats.get_mut_value(MAGICAL_ARMOR).current = 10;
        stats.get_mut_value(PHYSICAL_ARMOR).current = 20;
        assert_eq!(stats.get_armor_stat(true), 10);
        assert_eq!(stats.get_armor_stat(false), 20);
    }

    #[test]
    pub fn unit_stats_is_energy_stat() {
        assert!(Stats::is_energy_stat(HP));
        assert!(Stats::is_energy_stat(MANA));
        assert!(Stats::is_energy_stat(VIGOR));
        assert!(Stats::is_energy_stat(BERSERK));
        assert!(!Stats::is_energy_stat(SPEED));
    }

    #[test]
    pub fn unit_attribute() {
        let mut attr = Attribute {
            current: 10,
            max: 20,
            buf_effect_value: 5,
            buf_effect_percent: 10,
            buf_equip_value: 15,
            buf_equip_percent: 20,
            ..Default::default()
        };
        attr.sync_raw_values();
        assert_eq!(attr.current_raw, 10);
        assert_eq!(attr.max_raw, 20);
    }

    #[test]
    fn unit_is_dead() {
        let mut stats = Stats::default();
        stats.init();
        assert!(stats.is_dead().is_some());
        assert!(stats.is_dead().unwrap());
        stats.all_stats.get_mut(HP).unwrap().current = 15;
        assert!(!stats.is_dead().unwrap());
    }

    #[test]
    fn unit_set_current_stats() {
        let mut stats = Stats::default();
        stats.init();

        stats.modify_stat_current(HP, 10);
        assert_eq!(stats.all_stats[HP].current, 10);

        stats.modify_stat_current(HP, -5);
        assert_eq!(stats.all_stats[HP].current, 5);

        stats.modify_stat_current(HP, -10);
        assert_eq!(stats.all_stats[HP].current, 0);
    }

    #[test]
    fn unit_set_stats_on_effect() {
        let file_path = "./tests/offlines/characters/test.json"; // Path to the JSON file
        let c = Character::try_new_from_json(
            file_path,
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        );
        assert!(c.is_ok());
        let mut c = c.unwrap();
        c.stats.set_stats_on_effect(HP, -10, false, true);
        assert_eq!(125, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        c.stats.set_stats_on_effect(HP, 10, false, true);
        assert_eq!(135, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        c.stats.set_stats_on_effect(HP, 10, false, true);
        assert_eq!(145, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        c.stats.set_stats_on_effect(HP, -10, false, true);
        assert_eq!(135, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        c.stats.set_stats_on_effect(HP, 10, true, true);
        assert_eq!(148, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        c.stats.set_stats_on_effect(HP, -10, true, true);
        assert_eq!(135, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        // test raw max = 0, nothing change
        c.stats.all_stats[HP].max_raw = 0;
        assert_eq!(135, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        c.stats.set_stats_on_effect(DODGE, 20, false, true);
        assert_eq!(25, c.stats.all_stats[DODGE].max);
        assert_eq!(5, c.stats.all_stats[DODGE].current);
    }

    #[test]
    fn unit_apply_equipment_on_stats() {
        let stat = Attribute {
            current: 100,
            max: 100,
            max_raw: 100,
            ..Default::default()
        };
        let mut stats = Stats::default();
        stats.all_stats.insert(BERSERK.to_string(), stat.clone());
        let equipment = Equipment {
            stats: Stats {
                all_stats: vec![(
                    BERSERK.to_string(),
                    Attribute {
                        buf_equip_value: 10,
                        buf_equip_percent: 10,
                        ..Default::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..Default::default()
            },
            ..Default::default()
        };
        stats.apply_equipment_on_stats(&vec![equipment]);
        assert_eq!(120, stats.all_stats[BERSERK].max);
        assert_eq!(120, stats.all_stats[BERSERK].current);
        assert_eq!(100, stats.all_stats[BERSERK].max_raw);
        assert_eq!(10, stats.all_stats[BERSERK].buf_equip_value);
        assert_eq!(10, stats.all_stats[BERSERK].buf_equip_percent);
        assert_eq!(0, stats.all_stats[BERSERK].current_raw);
    }

    #[test]
    fn unit_init_aggro_on_turn() {
        let mut stats = Stats::default();
        stats.init();

        let mut all_aggro = HashMap::new();
        assert!(NB_TURN_SUM_AGGRO > 0);

        // Insert NB_TURN_SUM_AGGRO + 1 turns
        for i in 0..=NB_TURN_SUM_AGGRO {
            all_aggro.insert(i as u64, (i as i64 + 1) * 10);
        }

        // Initialize stats on turn NB_TURN_SUM_AGGRO + 1
        stats.init_aggro_on_turn(NB_TURN_SUM_AGGRO + 1, &all_aggro);

        // Compute expected sum: only the last NB_TURN_SUM_AGGRO turns
        let mut expected_sum = 0;
        for i in 1..=NB_TURN_SUM_AGGRO {
            expected_sum += (i as i64 + 1) * 10;
        }

        assert_eq!(expected_sum, stats.all_stats[AGGRO].current as i64);
    }
}
