#[cxx::bridge]
pub mod ffi {
    extern "Rust" {
        type ExtendedCharacter;
        /// Constructor
        pub fn try_new_ext_character() -> Box<ExtendedCharacter>;
        /// Getters
        pub fn get_is_random_target(&self) -> bool;
        pub fn get_is_heal_atk_blocked(&self) -> bool;
        pub fn get_is_first_round(&self) -> bool;
        /// Setters
        pub fn set_is_random_target(&mut self, value: bool);
        pub fn set_is_heal_atk_blocked(&mut self, value: bool);
        pub fn set_is_first_round(&mut self, value: bool);

    }
}

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
