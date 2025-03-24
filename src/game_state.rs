use std::collections::HashMap;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{
    character::{Character, CharacterType},
    common::{character_const::SPEED_THRESHOLD, stats_const::SPEED},
    players_manager::PlayerManager,
};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    /// Current turn number
    pub current_turn_nb: u64,
    /// Key turn number, value name
    pub died_ennemies: HashMap<i32, String>,
    /// List in the ascending order of the players
    pub order_to_play: Vec<String>,
    /// Current round number
    pub current_round: u64,
    /// Name of the game
    pub game_name: String,
    pub pm: PlayerManager,
    /// Current player of the round
    pub round_player: Character,
}

impl GameState {
    pub fn new(pm: PlayerManager) -> Self {
        GameState {
            pm,
            ..Default::default()
        }
    }
    pub fn start_game(&mut self) {
        // create name of exercise
        let time_str = crate::utils::get_current_time_as_string();
        self.game_name = format!("Game_{}", time_str);
    }
    pub fn start_new_turn(&mut self) -> Result<()> {
        // Increment turn number
        self.current_turn_nb += 1;
        // Reset to round 0
        self.current_round = 0;
        // Increment turn effects
        self.pm.increment_counter_effect();
        // Reset new round boolean for characters
        self.pm.reset_is_first_round();
        // Apply regen stats
        self.pm.apply_regen_stats(CharacterType::Boss);
        self.pm.apply_regen_stats(CharacterType::Hero);

        // For each turn now
        // Process the order of the players
        self.process_order_to_play();

        self.new_round()?;

        // TODO update game status
        // TODO init target view
        // TODO add channel for the logs

        Ok(())
    }

    pub fn process_order_to_play(&mut self) {
        // to be improved with stats
        // one player can play several times as well in different order
        self.order_to_play.clear();

        // sort by speed
        self.pm
            .all_heroes
            .sort_by(|a, b| a.stats.all_stats[SPEED].cmp(&b.stats.all_stats[SPEED]));
        let mut dead_heroes = Vec::new();
        for hero in &self.pm.all_heroes {
            if !hero.is_dead() {
                self.order_to_play.push(hero.name.clone());
            } else {
                dead_heroes.push(hero.name.clone());
            }
        }
        // add dead heroes
        for name in dead_heroes {
            self.order_to_play.push(name);
        }
        // add bosses
        // sort by speed
        self.pm
            .all_bosses
            .sort_by(|a, b| a.stats.all_stats[SPEED].cmp(&b.stats.all_stats[SPEED]));
        for boss in &self.pm.all_bosses {
            self.order_to_play.push(boss.name.clone());
        }
        // supplementariy atks to push
        self.add_sup_atk_turn(CharacterType::Hero);
        // self.add_sup_atk_turn(CharacterType::Boss, &mut self.order_to_play);
    }

    pub fn add_sup_atk_turn(&mut self, launcher_type: CharacterType) {
        let (player_list1, player_list2) = if launcher_type == CharacterType::Hero {
            (&mut self.pm.all_heroes, &self.pm.all_bosses)
        } else {
            (&mut self.pm.all_bosses, &self.pm.all_heroes)
        };
        for pl1 in player_list1 {
            if pl1.is_dead() {
                continue;
            }
            let speed_pl1 = match pl1.stats.all_stats.get_mut(SPEED) {
                Some(speed) => speed,
                None => continue,
            };
            for pl2 in player_list2 {
                let speed_pl2_current = pl2.stats.all_stats[SPEED].current;
                if speed_pl1.current - speed_pl2_current >= SPEED_THRESHOLD {
                    // Update of current value aspeed_threshold
                    speed_pl1.current = speed_pl1.current.saturating_sub(SPEED_THRESHOLD);
                    speed_pl1.max = speed_pl1.max.saturating_sub(SPEED_THRESHOLD);
                    speed_pl1.max_raw = speed_pl1.max_raw.saturating_sub(SPEED_THRESHOLD);
                    speed_pl1.current_raw = speed_pl1.current_raw.saturating_sub(SPEED_THRESHOLD);
                    self.order_to_play.push(pl1.name.clone());
                    break;
                }
            }
        }
    }

    /* bool GameDisplay::NewRound() {
    // Apply effects
    // Assess first round for the player
    // TODO create a method to do only on first round
    if (activePlayer->m_ExtCharacter != nullptr &&
        activePlayer->m_ExtCharacter->get_is_first_round()) {
      // update boolean
      activePlayer->m_ExtCharacter->set_is_first_round(false);

      // init aggro
      activePlayer->InitAggroOnTurn(gs->m_CurrentTurnNb);
      // Remove terminated effect
      const QStringList terminatedEffects =
          gm->m_PlayersManager->RemoveTerminatedEffectsOnPlayer(
              activePlayer->m_Name);
      emit SigUpdateChannelView(activePlayer->m_Name,
                                terminatedEffects.join("\n"),
                                activePlayer->color);

      // Update effect
      const QStringList effectsLogs = gm->m_PlayersManager->ApplyEffectsOnPlayer(
          activePlayer->m_Name, gm->m_GameState->m_CurrentTurnNb, false);
      emit SigUpdateChannelView(activePlayer->m_Name, effectsLogs.join("\n"),
                                activePlayer->color);
      // update buf pow
      // passive azrak TODO extract in a function
      if (activePlayer->m_Name == "Azrak Ombresang") {
        auto &localStat = activePlayer->m_Stats.m_AllStatsTable[STATS_POW_PHY];
        auto *phyBuf =
            activePlayer->m_AllBufs[static_cast<int>(BufTypes::powPhyBuf)];
        if (phyBuf != nullptr) {
          const auto &hpRxTable =
              activePlayer->m_LastTxRx[static_cast<int>(amountType::overHealRx)];
          int64_t hpRx = 0;
          if (hpRxTable.find(gs->m_CurrentTurnNb - 1) != hpRxTable.end()) {
            hpRx = hpRxTable.at(gs->m_CurrentTurnNb - 1);
          }
          // -phyBuf->get_value() : buf previous turn
          // hpRx : buf new turn
          QStringList azrakOverHeal;
          if (static_cast<int>(gs->m_CurrentTurnNb) - 2 > 0) {
            azrakOverHeal.append(QString("Overheal: Tour%1: -%2")
                                     .arg(gs->m_CurrentTurnNb - 2)
                                     .arg(phyBuf->get_value()));
          }
          azrakOverHeal.append(QString("Overheal: Tour%1: +%2")
                                   .arg(gs->m_CurrentTurnNb - 1)
                                   .arg(hpRx));
          azrakOverHeal.append(QString("Overheal total: Tour%1: %2")
                                   .arg(gs->m_CurrentTurnNb - 1)
                                   .arg(hpRx - phyBuf->get_value()));
          emit SigUpdateChannelView(activePlayer->m_Name,
                                    azrakOverHeal.join("\n"),
                                    activePlayer->color);
          const auto addValue = static_cast<int>(-phyBuf->get_value() + hpRx);
          Character::SetStatsOnEffect(localStat, addValue, false, true);
          phyBuf->set_buffers(hpRx, phyBuf->get_is_percent());
        }
      }

      // process actions on last turn damage received
      const auto &damageTx =
          activePlayer->m_LastTxRx[static_cast<int>(amountType::damageTx)];
      const bool isDamageTxLastTurn =
          damageTx.find(gs->m_CurrentTurnNb - 1) != damageTx.end();
      // passive power is_crit_heal_after_crit
      if (activePlayer->m_Power.is_crit_heal_after_crit && isDamageTxLastTurn &&
          activePlayer->m_isLastAtkCritical) {
        // in case of critical damage sent on last turn , next heal critical is
        // enable
        auto *buf =
            activePlayer
                ->m_AllBufs[static_cast<int>(BufTypes::nextHealAtkIsCrit)];
        if (buf != nullptr) {
          buf->set_is_passive_enabled(true);
        }
      }
      // passive power
      if (activePlayer->m_Power.is_damage_tx_heal_needy_ally &&
          isDamageTxLastTurn) {
        gm->m_PlayersManager->ProcessDamageTXHealNeedyAlly(
              activePlayer->m_Type, damageTx.at(gs->m_CurrentTurnNb - 1));
      }
    }

    // Assess boss atk
    if (const auto randAtkNb = activePlayer->GetRandomAtkNumber();
        randAtkNb.has_value()) {
      QStringList logTargetAtk;
      const auto randAtkStr =
          activePlayer->FormatStringRandAtk(randAtkNb.value());
      if (randAtkStr.has_value()) {
        logTargetAtk.append(randAtkStr.value());
      }
      // Choose the hero target with the most aggro in case of individual reach
      if (const auto heroAgg = gm->m_PlayersManager->GetHeroMostAggro();
          heroAgg.has_value()) {
        logTargetAtk.append(QString("%1 a le + d'aggro(%2)")
                                .arg(heroAgg->first)
                                .arg(heroAgg->second));
      }
      emit SigUpdateChannelView(activePlayer->m_Name, logTargetAtk.join("\n"),
                                activePlayer->color);
    } */
    pub fn new_round(&mut self) -> Result<()> {
        self.current_round += 1;
        if (self.current_round as usize) < self.order_to_play.len() {
            self.pm.update_current_player(
                self.current_turn_nb,
                &self.order_to_play[self.current_round as usize],
            )?;
        } else {
            bail!("No more player to play");
        }
        // TODO update game status
        // TODO channels for logss

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common::{character_const::SPEED_THRESHOLD, stats_const::SPEED},
        game_state::GameState,
        players_manager::PlayerManager,
    };

    #[test]
    fn unit_start_game() {
        let mut gs = GameState::default();
        gs.start_game();
        assert!(!gs.game_name.is_empty());
    }

    #[test]
    fn unit_start_new_turn() {
        let mut gs = GameState::new(PlayerManager::try_new("tests/characters").unwrap());
        assert!(gs.start_new_turn().is_ok());
        assert_eq!(gs.current_round, 1);
        assert_eq!(gs.current_turn_nb, 1);
    }

    #[test]
    fn unit_process_order_to_play() {
        let mut gs = GameState::new(PlayerManager::try_new("tests/characters").unwrap());
        let old_speed = gs.pm.all_heroes.first().cloned().unwrap().stats.all_stats[SPEED].clone();
        gs.process_order_to_play();
        let new_speed = gs.pm.all_heroes.first().cloned().unwrap().stats.all_stats[SPEED].clone();
        assert_eq!(gs.order_to_play.len(), 3);
        assert_eq!(gs.order_to_play[0], "Super test");
        assert_eq!(gs.order_to_play[1], "Boss1");
        // supplementary atk
        assert_eq!(gs.order_to_play[2], "Super test");

        assert_eq!(old_speed.current - SPEED_THRESHOLD, new_speed.current);
        assert_eq!(old_speed.max - SPEED_THRESHOLD, new_speed.max);
        assert_eq!(old_speed.max_raw - SPEED_THRESHOLD, new_speed.max_raw);
        assert_eq!(
            old_speed.current_raw - SPEED_THRESHOLD,
            new_speed.current_raw
        );
    }

    #[test]
    fn unit_add_sup_atk_turn() {
        let mut gs = GameState::new(PlayerManager::try_new("tests/characters").unwrap());
        let hero = gs.pm.all_heroes.first_mut().unwrap();
        hero.stats.all_stats.get_mut(SPEED).unwrap().current = 300;
        let boss = gs.pm.all_bosses.first_mut().unwrap();
        boss.stats.all_stats.get_mut(SPEED).unwrap().current = 10;
        gs.add_sup_atk_turn(crate::character::CharacterType::Hero);
        assert_eq!(gs.order_to_play.len(), 1);
    }
}
