use std::path::Path;

fn main() {
    let config_path = concat!(env!("HOME"), "/.config/critwm/config.rs");
    println!("cargo:rerun-if-changed={}", config_path);
    if Path::new(config_path).exists() {
        // If "$HOME/.config/critwm/config.rs" exists, pass custom_config option.
        // This means that this configuration file will be sourced instead of "src/config.def.rs".
        println!("cargo:rustc-cfg=feature=\"custom_config\"");
    }
}
