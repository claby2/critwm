#[macro_use]
extern crate log;

#[macro_use]
pub mod util;
pub mod backend;
pub mod error;
pub mod layouts;
pub mod socket;

pub mod config {
    // Parse configuration from user's filesystem if custom_config.
    #[cfg(feature = "custom_config")]
    include!(concat!(env!("HOME"), "/.config/critwm/config.rs"));

    // Fallback to default config in src if not custom_config.
    #[cfg(not(feature = "custom_config"))]
    include!("config.def.rs");
}
