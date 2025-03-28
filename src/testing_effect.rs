use crate::{
    common::{all_target_const::*, effect_const::*, reach_const::*},
    effect::EffectParam,
};

pub fn build_cooldown_effect() -> EffectParam {
    EffectParam {
        effect_type: EFFECT_NB_COOL_DOWN.to_owned(),
        nb_turns: 3,
        sub_value_effect: 0,
        target: TARGET_ALLY.to_owned(),
        reach: INDIVIDUAL.to_owned(),
        stats_name: "".to_owned(),
        value: 0,
        ..Default::default()
    }
}
