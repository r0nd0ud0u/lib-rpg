use std::{cmp::Ordering, collections::HashMap};

use serde::{Deserialize, Serialize};

use crate::common::stats_const::{
    AGGRO, AGGRO_RATE, BERSECK, BERSECK_RATE, CRITICAL_STRIKE, DODGE, HP, HP_REGEN, MAGICAL_ARMOR,
    MAGICAL_POWER, MANA, MANA_REGEN, PHYSICAL_ARMOR, PHYSICAL_POWER, SPEED, SPEED_REGEN, VIGOR,
    VIGOR_REGEN,
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
    pub buf_effect_value: u64,
    /// All buffer percentage are added in one percent value
    pub buf_effect_percent: u64,
    /// All buffer equipment are added in one value
    pub buf_equip_value: u64,
    /// All buffer equipment are added in one value
    pub buf_equip_percent: u64,
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

    pub all_stats: HashMap<String, Attribute>,
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
            .insert(BERSECK.to_string(), self.berseck.clone());
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
            .expect("HP key missing in all_stats")
    }
}
