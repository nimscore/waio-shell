# layer-shika

[Main repo](https://codeberg.org/waydeer/layer-shika) | [Mirror](https://git.dren.dog/waydeer/layer-shika) | [Temp mirror (github)](https://github.com/waydeerwm/layer-shika)

## ⚠️ WORK IN PROGRESS ⚠️

Oh deer! 🦌 You've stumbled upon `layer-shika`, a rust library crate that provides a layer shell implementation for wayland compositors, using slint for the GUI. It allows you to create overlay windows and panels or bars that integrate seamlessly with wayland-based desktop environments.

Please note that this library is currently in early development and is not yet ready for production use.

## Current Status

- **Architecture**: Clean hexagonal architecture with domain, adapters, and composition layers
- **Rendering**: EGL context management with `femtovg` renderer integration
- **Wayland**: Comprehensive layer shell protocol support using `smithay-client-toolkit`
  - Output management and configuration
  - Surface lifecycle management
  - Basic xdg popup support
  - Event handling system
- **UI Integration**: Slint integration layer with custom rendering backend
- **Documentation**: Work in progress
- **Examples**: Available - see [examples/](examples/) directory

**What's Working:**

- EGL context creation and management
- Wayland layer shell surface creation
- XDG popup support (with some limitations: resizing needed)
- Basic rendering with femtovg
- Output detection and management
- Event handling framework

**Known Limitations:**

- Not all features are fully implemented or tested
- API is still unstable and may change
- Comprehensive documentation pending

It's recommended to wait for a stable release before using this library in production projects. Development is trying to be as fast as a running skippy deer!

## Quick Start

Check out the [examples/](examples/) directory for comprehensive demonstrations.
Each example includes detailed documentation and can be run with:

```bash
cargo run -p simple-bar
cargo run -p multi-surface
cargo run -p declarative-config
...
...
```

See the [examples README](examples/README.md) for detailed usage instructions and patterns.

## Usage

If you want to use it now, use this repo as dependency instead of crates.io outdated versions.

## First Stable Release

It aim to have the first stable release by the end of 2025, with a focus on core functionality and API stability.

Stay tuned!

## Contributing

As the library is in a very early stage, it's recommended to open an issue to discuss ideas or proposed changes before submitting contributions. The project doesn't bite, it's not that kind of deer!
