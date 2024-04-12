#[cxx::bridge]
pub mod ffi {

    extern "Rust" {
        type Buffers;
        pub fn set_buffers(&mut self, value: i64, is_percent: bool);
        pub fn get_value(&self) -> i64;
        pub fn get_is_percent(&self) -> bool;
        pub fn set_is_passive_enabled(&mut self, value: bool);
        pub fn get_is_passive_enabled(&self) -> bool;
        pub fn buffers_new() -> Box<Buffers>;
    }
}

pub fn buffers_new() -> Box<Buffers> {
    Box::<Buffers>::default()
}

#[derive(Default, Debug, Clone)]
pub struct Buffers {
    /// A buf can be passive, that is without being a change of value
    pub is_passive_enabled: bool,
    /// If it is active, it changes the value
    pub value: i64,
    pub is_percent: bool,
}

impl Buffers {
    // Setters
    pub fn set_buffers(&mut self, value: i64, is_percent: bool) {
        self.value = value;
        self.is_percent = is_percent;
    }
    pub fn set_is_passive_enabled(&mut self, value: bool) {
        self.is_passive_enabled = value;
    }
    // Getters
    pub fn get_value(&self) -> i64 {
        self.value
    }
    pub fn get_is_percent(&self) -> bool {
        self.is_percent
    }
    pub fn get_is_passive_enabled(&self) -> bool {
        self.is_passive_enabled
    }
}
