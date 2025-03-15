use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct TxRx {
    #[serde(rename = "Tx-rx-size")]
    tx_rx_size: u64,
    #[serde(rename = "Tx-rx-type")]
    tx_rx_type: u64,
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
    pub aggro: Vec<Attribute>,

    #[serde(rename = "Aggro rate")]
    pub aggro_rate: Vec<Attribute>,

    #[serde(rename = "Magic armor")]
    pub magic_armor: Vec<Attribute>,

    #[serde(rename = "Physical armor")]
    pub physical_armor: Vec<Attribute>,

    #[serde(rename = "Magic strength")]
    pub magic_strength: Vec<Attribute>,

    #[serde(rename = "Physical strength")]
    pub physical_strength: Vec<Attribute>,

    #[serde(rename = "HP")]
    pub hp: Attribute,

    #[serde(rename = "Mana")]
    pub mana: Vec<Attribute>,

    #[serde(rename = "Vigor")]
    pub vigor: Vec<Attribute>,

    #[serde(rename = "Berseck")]
    pub berseck: Vec<Attribute>,

    #[serde(rename = "Berseck rate")]
    pub berseck_rate: Vec<Attribute>,

    #[serde(rename = "Speed")]
    pub speed: Vec<Attribute>,

    #[serde(rename = "Critical strike")]
    pub critical_strike: Vec<Attribute>,

    #[serde(rename = "Dodge")]
    pub dodge: Vec<Attribute>,

    #[serde(rename = "HP regeneration")]
    pub regeneration_hp: Vec<Attribute>,

    #[serde(rename = "Mana regeneration")]
    pub regeneration_mana: Vec<Attribute>,

    #[serde(rename = "Vigor regeneration")]
    pub regeneration_vigor: Vec<Attribute>,

    #[serde(rename = "Speed regeneration")]
    pub regeneration_speed: Vec<Attribute>,
}
