#[derive(Default, Debug, Clone)]
pub struct ExtendedCharacter {
    pub is_random_target: bool,
    pub is_heal_atk_blocked: bool,
    pub is_first_round: bool,
}

pub fn try_new_ext_character() -> Box<ExtendedCharacter> {
    Box::<ExtendedCharacter>::default()
}

impl ExtendedCharacter {
    /// Getters
    pub fn get_is_random_target(&self) -> bool {
        self.is_random_target
    }
    pub fn get_is_heal_atk_blocked(&self) -> bool {
        self.is_heal_atk_blocked
    }
    pub fn get_is_first_round(&self) -> bool {
        self.is_first_round
    }
    /// Setters
    pub fn set_is_random_target(&mut self, value: bool) {
        self.is_random_target = value;
    }
    pub fn set_is_heal_atk_blocked(&mut self, value: bool) {
        self.is_heal_atk_blocked = value;
    }
    pub fn set_is_first_round(&mut self, value: bool) {
        self.is_first_round = value;
    }
}


/*
  characType m_Type = characType::Hero;
  Stats m_Stats;
  std::unordered_map<QString, Stuff>
      m_WearingEquipment; // key: body, value: equipmentName
  std::unordered_map<QString, AttaqueType>
      m_AttakList; // key: attak name, value: AttakType struct
  // That vector contains all the atks from m_AttakList and is sorted by level.
  std::vector<AttaqueType> m_AtksByLevel;
  int m_Level = 1;
  int m_Exp = 0;
  int m_NextLevel = 100;
  std::vector<QString> m_Forms = std::vector<QString>{STANDARD_FORM};
  QString m_SelectedForm = STANDARD_FORM;
  QString m_ColorStr = "dark"; */

#[derive(Debug, Clone)]
pub struct Character {
    pub name: String,
    pub short_name: String,
    pub photo_name: String,
}

impl Default for Character {
    fn default() -> Self {
        Character {
            name:  String::from("default"),
            short_name: String::from("default"),
            photo_name: String::from("default"),
        }
    }
}