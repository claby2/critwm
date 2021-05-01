# critwm

`critwm` is a tiling window manager for X.

Features:

*   Compile-time configuration
*   Dynamic layout switching
*   Multiple monitor support (with `Xinerama`)
    *   Each monitor has 9 workspaces fixed to it.

`critwm` is not fully EWMH compliant.

## Dependencies

*   X11 (Xlib)
*   Xinerama

## Installation

### From source

    cargo install --path .

## Configuration

Configuration is done by editing the source code.
Most configuration can be done by just editing the configuration file (`src/config.rs`);
however, more advanced changes can be made by editing other files if needed.

### Layouts

`critwm` allows you to create a custom layout by adding a file to the layouts directory (`src/layouts/`).
Each layout should, at the very least, implement a function with the same parameters and return type as `crate::layouts::Layout`.
