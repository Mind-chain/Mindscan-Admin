#![allow(clippy::derive_partial_eq_without_eq)]

pub mod blockscout {
    pub mod admin {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/blockscout.admin.v1.rs"));
        }
    }
}
