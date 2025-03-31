use crate::target::TargetInfo;

pub fn build_target_boss_indiv() -> TargetInfo {
    TargetInfo {
        name: "Boss1".to_owned(),
        _is_targeted: false,
        _is_boss: true,
        _is_reach_rand: false,
    }
}
