use crate::target::TargetInfo;

pub fn build_target_boss_indiv() -> TargetInfo {
    TargetInfo {
        name: "Boss1".to_owned(),
        is_targeted: true,
        _is_boss: true,
        _is_reach_rand: false,
    }
}
