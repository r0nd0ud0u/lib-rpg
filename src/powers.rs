#[cxx::bridge]
pub mod ffi {
    pub struct Powers {
        /// Enables the critical of the next heal atk after a critical on damage atk
        pub is_crit_heal_after_crit: bool,
    }
    extern "Rust" {
    }
}
