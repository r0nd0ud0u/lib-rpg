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
    OverHealBoostStat,
    BoostedByHots,
    /// Effect to improve max value of a stat by value or percent (current value is updated by
    /// ratio). Which mode applies is decided by `Buffer.is_percent`.
    ChangeMaxStat,
    BlockHealAtk,
    /// Effect to improve current value of a stat by value or percent, decided by
    /// `Buffer.is_percent`.
    ChangeCurrentStat,
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
            BufKinds::OverHealBoostStat => "Overheal boosts stat",
            BufKinds::BoostedByHots => "Boosted by HoTs",
            BufKinds::ChangeMaxStat => "Max stat change",
            BufKinds::BlockHealAtk => "Heals blocked",
            BufKinds::ChangeCurrentStat => "Current stat change",
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
    fn unit_buf_kinds_display() {
        assert_eq!(format!("{}", BufKinds::DefaultBuf), "Default");
        assert_eq!(
            format!("{}", BufKinds::DamageRxPercent),
            "Damage received %"
        );
        assert_eq!(format!("{}", BufKinds::DamageTxPercent), "Damage dealt %");
        assert_eq!(format!("{}", BufKinds::HealTxPercent), "Healing dealt %");
        assert_eq!(format!("{}", BufKinds::HealRxPercent), "Healing received %");
        assert_eq!(format!("{}", BufKinds::DamageCritCapped), "Crit damage cap");
        assert_eq!(
            format!("{}", BufKinds::NextHealAtkIsCrit),
            "Next heal is critical"
        );
        assert_eq!(format!("{}", BufKinds::MultiValue), "Multiplier");
        assert_eq!(
            format!("{}", BufKinds::ApplyEffectInit),
            "Effect applications"
        );
        assert_eq!(
            format!("{}", BufKinds::OverHealBoostStat),
            "Overheal boosts stat"
        );
        assert_eq!(format!("{}", BufKinds::BoostedByHots), "Boosted by HoTs");
        assert_eq!(format!("{}", BufKinds::ChangeMaxStat), "Max stat change");
        assert_eq!(format!("{}", BufKinds::BlockHealAtk), "Heals blocked");
        assert_eq!(
            format!("{}", BufKinds::ChangeCurrentStat),
            "Current stat change"
        );
        assert_eq!(
            format!("{}", BufKinds::RepeatAsManyAsPossible),
            "Repeat attack"
        );
        assert_eq!(
            format!("{}", BufKinds::DecreasingRateOnTurn),
            "Decreasing rate"
        );
        assert_eq!(
            format!("{}", BufKinds::NbDecreasingByTurn),
            "Decreasing count"
        );
        assert_eq!(
            format!("{}", BufKinds::IsDamageTxHealNeedyAlly),
            "Damage converts to ally heal"
        );
        assert_eq!(format!("{}", BufKinds::CooldownTurnsNumber), "Cooldown");
        assert_eq!(format!("{}", BufKinds::ReinitBuf), "Effect reset");
        assert_eq!(format!("{}", BufKinds::RemoveOneDebuf), "Remove debuff");
        assert_eq!(
            format!("{}", BufKinds::BoostHotsByPercentage),
            "Boost HoTs %"
        );
        assert_eq!(
            format!("{}", BufKinds::BoostBufByHotsNumberInPercentage),
            "HoT stack bonus"
        );
        assert_eq!(
            format!("{}", BufKinds::PercentageIntoDamages),
            "Convert heal to damage"
        );
        assert_eq!(
            format!("{}", BufKinds::NextHealAtkIsCritical),
            "Next heal is critical"
        );
        assert_eq!(
            format!("{}", BufKinds::AddAsMuchAsHp),
            "Overheal stat boost"
        );
        assert_eq!(
            format!("{}", BufKinds::StreakBreakerCrit),
            "Streak breaker (crit)"
        );
        assert_eq!(
            format!("{}", BufKinds::StreakBreakerDodge),
            "Streak breaker (dodge)"
        );
        assert_eq!(
            format!("{}", BufKinds::ConditionDamagePrevTurn),
            "Condition: damage last turn"
        );
        assert_eq!(format!("{}", BufKinds::RepeatIfHeal), "Repeat if heal");
        assert_eq!(format!("{}", BufKinds::Resurrect), "Resurrect");
        assert_eq!(format!("{}", BufKinds::EnumSize), "—");
    }

    #[test]
    fn unit_update_buf() {
        let mut buff = Buffer::default();
        buff.update_buf(5, true, "HP");
        assert_eq!(buff.value, 5);
        assert!(buff.is_percent);
        assert_eq!(buff.stats_name, "HP");
        buff.update_buf(10, false, "Mana");
        assert_eq!(buff.value, 15);
    }

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
