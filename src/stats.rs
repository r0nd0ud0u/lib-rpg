use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct TxRx {
    #[serde(rename = "Tx-rx-size")]
    tx_rx_size: u64,
    #[serde(rename = "Tx-rx-type")]
    tx_rx_type: u64,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(default)]
struct Attribute {
    #[serde(rename = "Current")]
    current: u32,
    current_raw: u32,
    #[serde(rename = "Max")]
    max: u32,
    max_raw: u32,
    buf_effect_value: u32,
    buf_effect_percent: u32,
    buf_equip_value: u32,
    buf_equip_percent: u32,
}
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct Stats {
    #[serde(rename = "Aggro")]
    aggro: Vec<Attribute>,

    #[serde(rename = "Aggro rate")]
    aggro_rate: Vec<Attribute>,

    #[serde(rename = "Magic armor")]
    magic_armor: Vec<Attribute>,

    #[serde(rename = "Physical armor")]
    physical_armor: Vec<Attribute>,

    #[serde(rename = "Magic strength")]
    magic_strength: Vec<Attribute>,

    #[serde(rename = "Physical strength")]
    physical_strength: Vec<Attribute>,

    #[serde(rename = "HP")]
    hp: Vec<Attribute>,

    #[serde(rename = "Mana")]
    mana: Vec<Attribute>,

    #[serde(rename = "Vigor")]
    vigor: Vec<Attribute>,

    #[serde(rename = "Berseck")]
    berseck: Vec<Attribute>,

    #[serde(rename = "Berseck rate")]
    berseck_rate: Vec<Attribute>,

    #[serde(rename = "Speed")]
    speed: Vec<Attribute>,

    #[serde(rename = "Critical strike")]
    critical_strike: Vec<Attribute>,

    #[serde(rename = "Dodge")]
    dodge: Vec<Attribute>,

    #[serde(rename = "HP regeneration")]
    regeneration_hp: Vec<Attribute>,

    #[serde(rename = "Mana regeneration")]
    regeneration_mana: Vec<Attribute>,

    #[serde(rename = "Vigor regeneration")]
    regeneration_vigor: Vec<Attribute>,

    #[serde(rename = "Speed regeneration")]
    regeneration_speed: Vec<Attribute>,
}
