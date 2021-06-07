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

Configuring `critwm` is done by creating and editing the file:

    ~/.config/critwm/config.rs

If this file does not exist, [`src/config.def.rs`](./src/config.def.rs) will be used instead.

### Layouts

Custom layouts can be created by adding a file to the layouts directory [`src/layouts`](./src/layouts).
Each layout should implement a function with the same parameters and return type as `crate::layouts::LayoutFunc`.
