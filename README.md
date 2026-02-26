# Clux

Clux is a stacking Wayland compositor written in Rust. It takes the architectural foundation of the Smallvil reference and adapts it into a project focused on a spatial window overview and a decoupled X11 integration.

# Core Architecture

The defining trait of Clux is its "Satellite" approach to legacy compatibility. While the original Smallvil example typically embeds XWayland directly, clux is built to be X11-unaware.

By offloading X11 support to xwayland-satellite, the compositor remains lightweight and resilient. This separation of concerns means that X11 applications are treated as standard Wayland surfaces through a proxy, keeping the core codebase focused exclusively on modern protocols.

# Build the release binary

cargo build --release

# Run as a nested window for testing

WINIT_BACKEND=wayland ./target/release/clux
