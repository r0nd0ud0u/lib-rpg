#[cxx::bridge]
pub mod ffi {
    extern "Rust" {
        type ExtendedCharacter;
        /// Constructor
        pub fn try_new_ext_character() -> Box<ExtendedCharacter>;
        /// Getters
        pub fn get_is_random_target(&self) -> bool;
        /// Setters
        pub fn set_is_random_target(&mut self, value: bool);

    }
}

#[derive(Default, Debug, Clone)]
pub struct ExtendedCharacter {
    pub is_random_target: bool,
}

pub fn try_new_ext_character() -> Box<ExtendedCharacter> {
    Box::<ExtendedCharacter>::default()
}

impl ExtendedCharacter {
    /// Getters
    pub fn get_is_random_target(&self) -> bool {
        self.is_random_target
    }
    /// Setters
    pub fn set_is_random_target(&mut self, value: bool) {
        self.is_random_target = value;
    }
}
