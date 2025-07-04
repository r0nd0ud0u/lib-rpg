use serde::{Deserialize, Serialize};

pub enum BufTypes {
    DefaultBuf = 0,
    DamageRx,
    DamageTx,
    HealTx,
    HealRx,
    DamageCritCapped,
    PowPhyBuf,
    NextHealAtkIsCrit,
    MultiValue,
    ApplyEffectInit,
    ChangeByHealValue,
    BoostedByHots,
    EnumSize,
}

/// Returns: i64
/// Returns the buf/debuf on cur_value.
/// its type {percent, decimal} and the additional value
pub fn update_damage_by_buf(add_value: i64, is_percent: bool, cur_value: i64) -> i64 {
    if is_percent {
        // sign of cur_value taken into account
        cur_value * add_value / 100
    } else {
        let sign = if cur_value > 0 { 1 } else { -1 };
        sign * add_value
    }
}

/// Returns: i64
/// Multiply cur_value value by coeff_multi
pub fn update_heal_by_multi(cur_value: i64, coeff_multi: i64) -> i64 {
    cur_value * coeff_multi
}

/// Define the different state of a buf
#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct Buffers {
    /// A buf can be passive, that is without being a change of value
    #[serde(rename = "Buf-passive-enabled")]
    pub is_passive_enabled: bool,
    /// If it is active, it changes the value
    #[serde(rename = "Buf-value")]
    pub value: i64,
    /// Buf can be in percentage or in value
    #[serde(rename = "Buf-is-percent")]
    pub is_percent: bool,
    /// Potentially, a buffer can be applied on a stat, otherwise empty
    /// TODO: encode a list of string or try to decode with delimiter
    #[serde(rename = "Buf-all-stats")]
    pub all_stats_name: Vec<String>,
    /// TODO buf-type
    #[serde(rename = "Buf-type")]
    pub buf_type: i64,
}

impl Buffers {
    pub fn set_buffers(&mut self, value: i64, is_percent: bool) {
        self.value = value;
        self.is_percent = is_percent;
    }
}

#[cfg(test)]
mod tests {
    use crate::buffers::{update_heal_by_multi, Buffers};

    use super::update_damage_by_buf;

    #[test]
    pub fn unit_update_damage_by_buf() {
        // default buffer
        let result = update_damage_by_buf(0, false, 0);
        assert_eq!(result, 0);

        // buffer , decimal value
        let result = update_damage_by_buf(10, false, 20);
        assert_eq!(result, 10);

        // buffer , negative decimal value
        let result = update_damage_by_buf(-10, false, 20);
        assert_eq!(result, -10);

        // buffer , percent value
        let result = update_damage_by_buf(10, true, 100);
        assert_eq!(result, 10);

        // buffer , negative percent value
        let result = update_damage_by_buf(-10, true, 200);
        assert_eq!(result, -20);

        // negative amount
        let result = update_damage_by_buf(-10, false, -200);
        assert_eq!(result, 10);
        let result = update_damage_by_buf(-10, true, -200);
        assert_eq!(result, 20);
    }

    #[test]
    fn unit_update_heal_by_multi() {
        let result = update_heal_by_multi(10, 0);
        assert_eq!(0, result);

        let result = update_heal_by_multi(10, 10);
        assert_eq!(100, result);
    }

    #[test]
    fn unit_set_buffers() {
        let mut buff = Buffers::default();
        buff.set_buffers(10, false);
        assert_eq!(buff.is_percent, false);
        assert_eq!(buff.buf_type, 0);
        assert_eq!(buff.all_stats_name.is_empty(), true);
        assert_eq!(buff.is_passive_enabled, false);
        assert_eq!(buff.value, 10);

        buff.set_buffers(20, true);
        assert_eq!(buff.is_percent, true);
        assert_eq!(buff.buf_type, 0);
        assert_eq!(buff.all_stats_name.is_empty(), true);
        assert_eq!(buff.is_passive_enabled, false);
        assert_eq!(buff.value, 20);
    }
}
