use crate::{
    attack_type::AttackType,
    buffers::{BufTypes, Buffers, update_damage_by_buf, update_heal_by_multi},
    common::{all_target_const::TARGET_ENNEMY, attak_const::COEFF_CRIT_DMG, stats_const::HP},
    effect::{self, EffectParam},
    players_manager::{DodgeInfo, GameAtkEffects},
    target::is_target_ally,
};
use std::collections::HashMap;

/// CharacterRoundsInfo
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub struct CharacterRoundsInfo {
    /// Fight information: Is the random character targeted by the current attack of other character
    #[serde(default, rename = "is_random_target")]
    pub is_random_target: bool,
    /// Fight information: TODO is_heal_atk_blocked
    #[serde(default, rename = "is_heal_atk_blocked")]
    pub is_heal_atk_blocked: bool,
    /// Fight information: Playing the first round of that tour
    #[serde(default, rename = "is_first_round")]
    pub is_first_round: bool,
    #[serde(default, rename = "launchable_atks")]
    pub launchable_atks: Vec<AttackType>,
    /// Fight information: dodge information on atk
    #[serde(default, rename = "dodge-info")]
    pub dodge_info: DodgeInfo,
    /// Fight information: nb-actions-in-round
    #[serde(default, rename = "nb-actions-in-round")]
    pub actions_done_in_round: u64,
    /// Fight information: is_current_target
    #[serde(default, rename = "is-current-target")]
    pub is_current_target: bool,
    /// Fight information: damages transmitted or received through the fight
    #[serde(default, rename = "Tx-rx")]
    pub tx_rx: Vec<HashMap<u64, i64>>,
    /// Fight information: Enabled buf/debuf acquired through the fight
    #[serde(default, rename = "Buf-debuf")]
    pub all_buffers: Vec<Buffers>,
    #[serde(default, rename = "ExpToNextLevel")]
    /// Experience to acquire to upgrade to next level
    pub exp_to_next_level: u64,
    /// Experience of the character, start 0
    #[serde(default, rename = "Experience")]
    pub exp: u64,
    /// Potential target by an individual effect of an atk
    #[serde(default, rename = "is-potential-target")]
    pub is_potential_target: bool,
}

impl Default for CharacterRoundsInfo {
    fn default() -> Self {
        let mut info = CharacterRoundsInfo {
            is_random_target: false,
            is_heal_atk_blocked: false,
            is_first_round: true,
            launchable_atks: Vec::new(),
            dodge_info: DodgeInfo::default(),
            actions_done_in_round: 0,
            is_current_target: false,
            tx_rx: vec![HashMap::new()],
            all_buffers: vec![],
            exp_to_next_level: 100,
            exp: 0,
            is_potential_target: false,
        };
        // init all_buffers
        info.new_buffers();

        info
    }
}

impl CharacterRoundsInfo {
    pub fn apply_launchable_atks(&mut self, launchable_atks: Vec<AttackType>) {
        self.launchable_atks = launchable_atks;
    }
}

/// CharacterRoundsInfo
#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub struct HotsBufs {
    pub hot_nb: u64,
    pub dot_nb: u64,
    pub buf_nb: u64,
    pub debuf_nb: u64,
    pub hot_txt: Vec<String>,
    pub dot_txt: Vec<String>,
    pub buf_txt: Vec<String>,
    pub debuf_txt: Vec<String>,
}
impl CharacterRoundsInfo {
    /// Init all buffers with default values
    pub fn new_buffers(&mut self) {
        for _ in 0..BufTypes::EnumSize as usize {
            self.all_buffers.push(Buffers::default());
        }
    }

    /// Output: hot, dot, buf, debuf
    pub fn get_hot_and_buf_nbs_txts(all_effects: &Vec<GameAtkEffects>) -> HotsBufs {
        let mut hots_bufs = HotsBufs::default();
        for e in all_effects {
            if e.all_atk_effects.input_effect_param.nb_turns < 2 {
                continue;
            }
            let txt = Self::get_hot_and_buf_texts(&e.all_atk_effects.input_effect_param);
            if effect::is_hot(
                &e.all_atk_effects.input_effect_param.effect_type,
                &e.all_atk_effects.input_effect_param.stats_name,
                e.all_atk_effects.input_effect_param.value,
            ) {
                hots_bufs.hot_nb += 1;
                hots_bufs.hot_txt.push(txt);
            } else if e.all_atk_effects.input_effect_param.stats_name == HP {
                hots_bufs.dot_nb += 1;
                hots_bufs.dot_txt.push(txt);
            } else if e.all_atk_effects.input_effect_param.value > 0 {
                hots_bufs.buf_nb += 1;
                hots_bufs.buf_txt.push(txt);
            } else {
                hots_bufs.debuf_nb += 1;
                hots_bufs.debuf_txt.push(txt);
            }
        }
        hots_bufs
    }

    fn get_hot_and_buf_texts(ep: &EffectParam) -> String {
        if ep.stats_name.is_empty() {
            format!("{}: {}", ep.effect_type, ep.value)
        } else {
            format!("{}-{}: {}", ep.effect_type, ep.stats_name, ep.value)
        }
    }

    pub fn is_dodging(&self, target_kind: &str) -> bool {
        self.dodge_info.is_dodging && target_kind == TARGET_ENNEMY
    }

    pub fn is_blocking(&mut self, ep: &EffectParam) -> bool {
        self.dodge_info.is_blocking && ep.stats_name == HP && ep.target_kind == TARGET_ENNEMY
    }

    pub fn apply_buf_debuf(&self, full_amount: i64, target: &str, is_crit: bool) -> i64 {
        let mut real_amount = full_amount;
        let mut buf_debuf = 0;
        let mut coeff_crit = COEFF_CRIT_DMG;
        // buf debuf heal
        if full_amount > 0 && is_target_ally(target) {
            // Launcher TX: BufTypes::MultiValue
            // To place first
            if let Some(buf_multi) = self.all_buffers.get(BufTypes::MultiValue as usize)
                && buf_multi.value > 0
            {
                real_amount = update_heal_by_multi(real_amount, buf_multi.value);
            }
            // Launcher TX: BufTypes::HealTx
            if let Some(buf_hp_tx) = self.all_buffers.get(BufTypes::HealTx as usize) {
                buf_debuf +=
                    update_damage_by_buf(buf_hp_tx.value, buf_hp_tx.is_percent, real_amount);
            }
            // Receiver RX: BufTypes::HealRx
            if let Some(buf_hp_rx) = self.all_buffers.get(BufTypes::HealRx as usize) {
                buf_debuf +=
                    update_damage_by_buf(buf_hp_rx.value, buf_hp_rx.is_percent, real_amount);
            }
            // Launcher TX: BufTypes::BoostedByHots
            if let Some(buf_nb_hots) = self.all_buffers.get(BufTypes::BoostedByHots as usize) {
                buf_debuf +=
                    update_damage_by_buf(buf_nb_hots.value, buf_nb_hots.is_percent, real_amount);
            }
        }
        // buf debuf damage
        if full_amount < 0 && !is_target_ally(target) {
            // Launcher TX: BufTypes::DamageTx
            if let Some(buf_dmg_tx) = self.all_buffers.get(BufTypes::DamageTx as usize) {
                buf_debuf +=
                    update_damage_by_buf(buf_dmg_tx.value, buf_dmg_tx.is_percent, real_amount);
            }
            // Receiver RX: BufTypes::DamageRx
            if let Some(buf_dmg_rx) = self.all_buffers.get(BufTypes::DamageRx as usize) {
                buf_debuf +=
                    update_damage_by_buf(buf_dmg_rx.value, buf_dmg_rx.is_percent, real_amount);
            }
            // Receiver RX: BufTypes::DamageCritCapped
            if let Some(buf_dmg_crit) = self.all_buffers.get(BufTypes::DamageCritCapped as usize) {
                // improve crit coeff
                coeff_crit += buf_dmg_crit.value as f64;
            }
        }

        // apply buf/debuf
        real_amount += buf_debuf;
        // is it a critical strike ?
        if is_crit {
            real_amount = (real_amount as f64 * coeff_crit).round() as i64;
        }

        real_amount
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        buffers::BufTypes,
        character_mod::rounds_information::{CharacterRoundsInfo, HotsBufs},
        common::all_target_const::{TARGET_ALLY, TARGET_ENNEMY},
        players_manager::GameAtkEffects,
        testing_effect::{
            build_buf_effect_individual, build_debuf_effect_individual,
            build_dmg_effect_individual, build_dot_effect_individual, build_hot_effect_individual,
        },
    };

    #[test]
    fn unit_get_hot_and_buf_nbs() {
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&vec![]);
        assert_eq!(result, HotsBufs::default());
        let mut all_effects: Vec<GameAtkEffects> = vec![];
        // add a 1-turn-effect
        all_effects.push(GameAtkEffects {
            all_atk_effects: build_dmg_effect_individual(),
            ..Default::default()
        });
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(result, HotsBufs::default());
        // add a 2-turn-effect HOT
        all_effects.push(GameAtkEffects {
            all_atk_effects: build_hot_effect_individual(),
            ..Default::default()
        });
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(
            result,
            HotsBufs {
                hot_nb: 1,
                dot_nb: 0,
                buf_nb: 0,
                debuf_nb: 0,
                hot_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::HP,
                    30
                )],
                dot_txt: vec![],
                buf_txt: vec![],
                debuf_txt: vec![]
            }
        );
        // add a 3-turn-effect DOT
        all_effects.push(GameAtkEffects {
            all_atk_effects: build_dot_effect_individual(),
            ..Default::default()
        });
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(
            result,
            HotsBufs {
                hot_nb: 1,
                dot_nb: 1,
                buf_nb: 0,
                debuf_nb: 0,
                hot_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::HP,
                    30
                )],
                dot_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::HP,
                    -20
                )],
                buf_txt: vec![],
                debuf_txt: vec![]
            }
        );
        // add a 3-turn-effect DOT
        all_effects.push(GameAtkEffects {
            all_atk_effects: build_buf_effect_individual(),
            ..Default::default()
        });
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(
            result,
            HotsBufs {
                hot_nb: 1,
                dot_nb: 1,
                buf_nb: 1,
                debuf_nb: 0,
                hot_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::HP,
                    30
                )],
                dot_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::HP,
                    -20
                )],
                buf_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::MAGICAL_ARMOR,
                    20
                )],
                debuf_txt: vec![]
            }
        );
        // add a 3-turn-effect DOT
        all_effects.push(GameAtkEffects {
            all_atk_effects: build_debuf_effect_individual(),
            ..Default::default()
        });
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(
            result,
            HotsBufs {
                hot_nb: 1,
                dot_nb: 1,
                buf_nb: 1,
                debuf_nb: 1,
                hot_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::HP,
                    30
                )],
                dot_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::HP,
                    -20
                )],
                buf_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::MAGICAL_ARMOR,
                    20
                )],
                debuf_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::MAGICAL_ARMOR,
                    -20
                )]
            }
        );
    }

    #[test]
    fn unit_apply_buf_debuf() {
        let mut cri = CharacterRoundsInfo::default();
        // no buf/debuf
        let result = cri.apply_buf_debuf(100, TARGET_ALLY, false);
        assert_eq!(result, 100);

        // buf defub damage against ennemy

        // Launcher TX: BufTypes::DamageTx
        // damage buf aigainst ennemy
        cri.all_buffers
            .get_mut(BufTypes::DamageTx as usize)
            .unwrap()
            .set_buffers(20, false);
        let result = cri.apply_buf_debuf(-100, TARGET_ENNEMY, false);
        // -100 -20 = -120
        assert_eq!(result, -120);
        // same but with critical strike
        let result = cri.apply_buf_debuf(-100, TARGET_ENNEMY, true);
        // -100 -20 = -120 * 2 = -240
        assert_eq!(result, -240);
        cri.all_buffers
            .get_mut(BufTypes::DamageTx as usize)
            .unwrap()
            .set_buffers(0, false);

        //Receiver RX: BufTypes::DamageRx
        cri.all_buffers
            .get_mut(BufTypes::DamageRx as usize)
            .unwrap()
            .set_buffers(20, false);
        let result = cri.apply_buf_debuf(-100, TARGET_ENNEMY, false);
        // -100 -20 = -120
        assert_eq!(result, -120);
        cri.all_buffers
            .get_mut(BufTypes::DamageRx as usize)
            .unwrap()
            .set_buffers(0, false);

        //Receiver RX: BufTypes::DamageCritCapped
        cri.all_buffers
            .get_mut(BufTypes::DamageCritCapped as usize)
            .unwrap()
            .set_buffers(2, false);
        // crit is doubled init:2 -> 2 + 2 = 4
        let result = cri.apply_buf_debuf(-100, TARGET_ENNEMY, true);
        // -100 * 4 = -400
        assert_eq!(result, -400);
        // it can be accunulated with damage buf
        cri.all_buffers
            .get_mut(BufTypes::DamageTx as usize)
            .unwrap()
            .set_buffers(20, false);
        let result = cri.apply_buf_debuf(-100, TARGET_ENNEMY, true);
        // -100 -20 = -120* 4 = -480
        assert_eq!(result, -480);
        cri.all_buffers
            .get_mut(BufTypes::DamageCritCapped as usize)
            .unwrap()
            .set_buffers(0, false);
        cri.all_buffers
            .get_mut(BufTypes::DamageTx as usize)
            .unwrap()
            .set_buffers(0, false);

        // buf debuf heal against ally

        // Launcher TX: BufTypes::MultiValue
        cri.all_buffers
            .get_mut(BufTypes::MultiValue as usize)
            .unwrap()
            .set_buffers(3, false);
        let result = cri.apply_buf_debuf(100, TARGET_ALLY, false);
        // 100 * 3 = 300
        assert_eq!(result, 300);
        cri.all_buffers
            .get_mut(BufTypes::MultiValue as usize)
            .unwrap()
            .set_buffers(0, false);

        // BufTypes::HealTx
        cri.all_buffers
            .get_mut(BufTypes::HealTx as usize)
            .unwrap()
            .set_buffers(20, false);
        let result = cri.apply_buf_debuf(100, TARGET_ALLY, false);
        // 100 + 20 = 120
        assert_eq!(result, 120);
        cri.all_buffers
            .get_mut(BufTypes::HealTx as usize)
            .unwrap()
            .set_buffers(0, false);
        // BufTypes::HealRx
        cri.all_buffers
            .get_mut(BufTypes::HealRx as usize)
            .unwrap()
            .set_buffers(20, false);
        let result = cri.apply_buf_debuf(100, TARGET_ALLY, false);
        // 100 + 20 = 120
        assert_eq!(result, 120);
        cri.all_buffers
            .get_mut(BufTypes::HealRx as usize)
            .unwrap()
            .set_buffers(0, false);
        // BufTypes::BoostedByHots
        cri.all_buffers
            .get_mut(BufTypes::BoostedByHots as usize)
            .unwrap()
            .set_buffers(20, false);
        let result = cri.apply_buf_debuf(100, TARGET_ALLY, false);
        // 100 + 20 = 120
        assert_eq!(result, 120);
        cri.all_buffers
            .get_mut(BufTypes::BoostedByHots as usize)
            .unwrap()
            .set_buffers(0, false);
    }
}
