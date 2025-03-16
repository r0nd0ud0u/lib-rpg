use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct TxRx {
    #[serde(rename = "Tx-rx-size")]
    pub tx_rx_size: u64,
    #[serde(rename = "Tx-rx-type")]
    pub tx_rx_type: u64,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct Attribute {
    #[serde(rename = "Current")]
    pub current: u32,
    pub current_raw: u32,
    #[serde(rename = "Max")]
    pub max: u32,
    pub max_raw: u32,
    pub buf_effect_value: u32,
    pub buf_effect_percent: u32,
    pub buf_equip_value: u32,
    pub buf_equip_percent: u32,
}
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
