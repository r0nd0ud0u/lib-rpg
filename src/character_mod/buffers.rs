use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BufKinds {
    #[default]
    DefaultBuf,
    DamageRxPercent,
    DamageTxPercent,
    HealTxPercent,
    HealRxPercent,
    DamageCritCapped,
    NextHealAtkIsCrit,
    MultiValue,
    ApplyEffectInit,
    ChangeByHealValue,
    BoostedByHots,
    /// Effect to improve max value of a stat by percent (current value is updated by ratio)
    ChangeMaxStatByPercentage,
    /// Effect to improve max value of a stat by value (current value is updated by ratio)
    ChangeMaxStatByValue,
    BlockHealAtk,
    /// Effect to improve current value of a stat by value
    ChangeCurrentStatByValue,
    /// Effect to improve current value of a stat by percent
    ChangeCurrentStatByPercentage,
    /// Assess the amount of applies for a stat
    RepeatAsManyAsPossible,
    /// Effect to execute an atk with a decreasing success rate defined by a step on effect value
    DecreasingRateOnTurn,
    NbDecreasingByTurn,
    /// Enables the power to heal the most needy ally using damage tx of previous turn
    IsDamageTxHealNeedyAlly,
    CooldownTurnsNumber,
    ReinitBuf,
    RemoveOneDebuf,
    BoostHotsByPercentage,
    BoostBufByHotsNumberInPercentage,
    PercentageIntoDamages,
    NextHealAtkIsCritical,
    AddAsMuchAsHp,
    /// Enables the crit streak-breaker: value = max consecutive turns without a crit
    /// before the next attack is guaranteed to crit. Applied by effects or active by rank/class/level.
    StreakBreakerCrit,
    /// Enables the dodge streak-breaker: value = max consecutive turns without a dodge
    /// before the next attack is guaranteed to dodge/block.
    StreakBreakerDodge,
    /// Gate condition: all subsequent effects are skipped unless the character
    /// dealt damage on the previous turn.
    ConditionDamagePrevTurn,
    /// Repeat the attack with a given % chance if the character healed on the previous turn.
    RepeatIfHeal,
    /// Revive a dead character and restore a fixed amount of HP.
    Resurrect,
    EnumSize,
}

impl fmt::Display for BufKinds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BufKinds::DefaultBuf => "Default",
            BufKinds::DamageRxPercent => "Damage received %",
            BufKinds::DamageTxPercent => "Damage dealt %",
            BufKinds::HealTxPercent => "Healing dealt %",
            BufKinds::HealRxPercent => "Healing received %",
            BufKinds::DamageCritCapped => "Crit damage cap",
            BufKinds::NextHealAtkIsCrit => "Next heal is critical",
            BufKinds::MultiValue => "Multiplier",
            BufKinds::ApplyEffectInit => "Effect applications",
            BufKinds::ChangeByHealValue => "Stat boosted by heal",
            BufKinds::BoostedByHots => "Boosted by HoTs",
            BufKinds::ChangeMaxStatByPercentage => "Max stat change %",
            BufKinds::ChangeMaxStatByValue => "Max stat change",
            BufKinds::BlockHealAtk => "Heals blocked",
            BufKinds::ChangeCurrentStatByValue => "Current stat change",
            BufKinds::ChangeCurrentStatByPercentage => "Current stat change %",
            BufKinds::RepeatAsManyAsPossible => "Repeat attack",
            BufKinds::DecreasingRateOnTurn => "Decreasing rate",
            BufKinds::NbDecreasingByTurn => "Decreasing count",
            BufKinds::IsDamageTxHealNeedyAlly => "Damage converts to ally heal",
            BufKinds::CooldownTurnsNumber => "Cooldown",
            BufKinds::ReinitBuf => "Effect reset",
            BufKinds::RemoveOneDebuf => "Remove debuff",
            BufKinds::BoostHotsByPercentage => "Boost HoTs %",
            BufKinds::BoostBufByHotsNumberInPercentage => "HoT stack bonus",
            BufKinds::PercentageIntoDamages => "Convert heal to damage",
            BufKinds::NextHealAtkIsCritical => "Next heal is critical",
            BufKinds::AddAsMuchAsHp => "Overheal stat boost",
            BufKinds::StreakBreakerCrit => "Streak breaker (crit)",
            BufKinds::StreakBreakerDodge => "Streak breaker (dodge)",
            BufKinds::ConditionDamagePrevTurn => "Condition: damage last turn",
            BufKinds::RepeatIfHeal => "Repeat if heal",
            BufKinds::Resurrect => "Resurrect",
            BufKinds::EnumSize => "—",
        };
        write!(f, "{}", s)
    }
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
pub struct Buffer {
    /// A buf can be passive, that is without being a change of value
    #[serde(rename = "passive-enabled")]
    pub is_passive_enabled: bool,
    #[serde(rename = "passive")]
    pub is_passive: bool,
    /// If it is active, it changes the value
    #[serde(rename = "value")]
    pub value: i64,
    /// Buf can be in percentage or in value
    #[serde(rename = "is-percent")]
    pub is_percent: bool,
    /// Potentially, a buffer can be applied on a stat, otherwise empty
    #[serde(rename = "stats-name")]
    pub stats_name: String,
    /// buf-type
    #[serde(rename = "kind")]
    pub kind: BufKinds,
}

impl Buffer {
    pub fn set_buffers(&mut self, value: i64, is_percent: bool) {
        self.value = value;
        self.is_percent = is_percent;
    }

    pub fn update_buf(&mut self, value: i64, is_percent: bool, stat: &str) {
        self.value += value;
        self.is_percent = is_percent;
        self.stats_name = stat.to_owned();
    }
}

#[cfg(test)]
mod tests {
    use crate::character_mod::buffers::{BufKinds, Buffer, update_heal_by_multi};

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
        let mut buff = Buffer::default();
        buff.set_buffers(10, false);
        assert!(!buff.is_percent);
        assert_eq!(buff.kind, BufKinds::DefaultBuf);
        assert!(buff.stats_name.is_empty());
        assert!(!buff.is_passive_enabled);
        assert_eq!(buff.value, 10);

        buff.set_buffers(20, true);
        assert!(buff.is_percent);
        assert_eq!(buff.kind, BufKinds::DefaultBuf);
        assert!(buff.stats_name.is_empty());
        assert!(!buff.is_passive_enabled);
        assert_eq!(buff.value, 20);
    }
}
