# layer-shika Examples

This directory contains comprehensive examples demonstrating the key features and use cases of layer-shika.

## Quick Start

Each example is a standalone crate that can be run from anywhere in the workspace:

```bash
# From workspace root
cargo run -p simple-bar
cargo run -p multi-surface
cargo run -p declarative-config

# Or with logging
RUST_LOG=info cargo run -p simple-bar

# Or from the example directory
cd examples/simple-bar
cargo run
```

## Example Progression

**Recommended learning path:**

1. **simple-bar** - Start here to understand the basics
2. **multi-surface** - Learn about multiple surfaces and callbacks
3. **declarative-config** - See the alternative configuration approach
4. **event-loop** - Explore event loop integration with timers and channels

## Common Patterns

### UI Files

Each example includes `.slint` files in its `ui/` directory. These demonstrate:

- Window components for surfaces
- Property bindings for dynamic updates
- Callback definitions for event handling

### Error Handling

All examples use layer-shika's `Result<()>` type for error handling with the `?` operator.

## Coming Soon

Additional examples demonstrating:
- Event loop integration (timers, channels, custom event sources)
- Multi-output support (multiple monitors) with different surfaces per output
- Advanced popup patterns
- Dynamic UI loading

## Contributing Examples

When adding new examples:

1. Create a new crate in `examples/<name>/`
2. Add to workspace members in root `Cargo.toml`
3. Include `Cargo.toml`, `src/main.rs`, `ui/*.slint`, and `README.md`
4. Follow the naming convention: kebab-case
5. Add entry to this README with clear description
6. Ensure code passes `cargo clippy --all-targets`
