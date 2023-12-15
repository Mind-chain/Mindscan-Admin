#![allow(clippy::derive_partial_eq_without_eq)]
pub mod blockscout {
    pub mod contracts_info {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/blockscout.contracts_info.v1.rs"));
        }
    }
}
