use crate::common::{effect_const::TARGET_ENNEMY, stats_const::STATS_HP};
pub fn is_heal_effect(stats_name: &str, target_reach: &str) -> bool {
    if target_reach != TARGET_ENNEMY && stats_name == STATS_HP {
        return true;
    }
    false
}
