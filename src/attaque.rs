use crate::common::{all_target_const::TARGET_ENNEMY, stats_const::HP};

/// Return true if changing stats is HP and if it is from an ally
/// Otherwise false.
pub fn is_heal_effect(stats_name: &str, target_reach: &str) -> bool {
    if target_reach != TARGET_ENNEMY && stats_name == HP {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::{
        attaque::is_heal_effect,
        common::{
            all_target_const::{TARGET_ALLY, TARGET_ENNEMY},
            stats_const::HP,
        },
    };

    #[test]
    fn unit_is_heal_effect() {
        let result = is_heal_effect(HP, TARGET_ENNEMY);
        assert!(!result);

        let result = is_heal_effect(HP, TARGET_ALLY);
        assert!(result);
    }
}
