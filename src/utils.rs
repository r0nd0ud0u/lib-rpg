use utils::Shared;

#[cxx::bridge]
mod utils {
    #[derive(Clone)]
    struct Shared {
        v: u32,
    }
    extern "Rust" {
        fn rusty_cxxbridge_vector()-> Vec<Shared>;
    }
}

pub fn rusty_cxxbridge_vector()-> Vec<Shared> {
  [Shared{ v: 32 }].to_vec()
}