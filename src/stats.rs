use serde::{Deserialize, Serialize};

/// Define allt the paramaters of tx-rx
#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct TxRx {
    /// TODO use?
    #[serde(rename = "Tx-rx-size")]
    pub tx_rx_size: u64,
    /// TODO enum
    #[serde(rename = "Tx-rx-type")]
    pub tx_rx_type: u64,
}

/// Define all the parameter of an attribute of a stat
#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct Attribute {
    /// Current value of the stat, with equipment and buf/debuf included
    #[serde(rename = "Current")]
    pub current: u32,
    /// Current raw value of the stat, WITHOUT equipment and buf/debuf included
    pub current_raw: u32,
    /// Max value of the stat, with equipment and buf/debuf included
    #[serde(rename = "Max")]
    pub max: u32,
    /// Raw Max value of the stat, WITHOUT equipment and buf/debuf included
    pub max_raw: u32,
    /// All buffer values are added in one value
    pub buf_effect_value: u32,
    /// All buffer percentage are added in one percent value
    pub buf_effect_percent: u32,
    /// All buffer equipment are added in one value
    pub buf_equip_value: u32,
    /// All buffer equipment are added in one value
    pub buf_equip_percent: u32,
}

/// Define all the parameters of the stats of one character
#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct Stats {
    #[serde(rename = "Aggro")]
    pub aggro: Attribute,

    #[serde(rename = "Aggro rate")]
    pub aggro_rate: Attribute,

    #[serde(rename = "Magical armor")]
    pub magical_armor: Attribute,

    #[serde(rename = "Physical armor")]
    pub physical_armor: Attribute,

    #[serde(rename = "Magical power")]
    pub magic_power: Attribute,

    #[serde(rename = "Physical power")]
    pub physical_power: Attribute,

    #[serde(rename = "HP")]
    pub hp: Attribute,

    #[serde(rename = "Mana")]
    pub mana: Attribute,

    #[serde(rename = "Vigor")]
    pub vigor: Attribute,

    #[serde(rename = "Berseck")]
    pub berseck: Attribute,

    #[serde(rename = "Berseck rate")]
    pub berseck_rate: Attribute,

    #[serde(rename = "Speed")]
    pub speed: Attribute,

    #[serde(rename = "Critical strike")]
    pub critical_strike: Attribute,

    #[serde(rename = "Dodge")]
    pub dodge: Attribute,

    #[serde(rename = "HP regeneration")]
    pub hp_regeneration: Attribute,

    #[serde(rename = "Mana regeneration")]
    pub mana_regeneration: Attribute,

    #[serde(rename = "Vigor regeneration")]
    pub vigor_regeneration: Attribute,

    #[serde(rename = "Speed regeneration")]
    pub speed_regeneration: Attribute,
}
