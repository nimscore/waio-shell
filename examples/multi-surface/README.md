# Multi-Surface Example

This example demonstrates creating a shell with multiple independent surfaces - a top bar and a bottom dock.

## What it demonstrates

- Multiple surface creation in a single shell
- Chaining surface configurations with `.surface()`
- Different anchoring for each surface (top and bottom)
- Independent exclusive zones per surface
- Separate namespaces for each surface
- Registering callbacks on specific surfaces
- Listing all surfaces with `shell.surface_names()`

## Running

```bash
cd examples/multi-surface
RUST_LOG=info cargo run
```

## Key concepts

Each surface is configured independently via chained `.surface()` calls:

```rust
let mut shell = Shell::from_file("ui/shell.slint")
    .surface("TopBar")
        .height(42)
        .anchor(AnchorEdges::top_bar())
        .exclusive_zone(42)
    .surface("Dock")
        .height(64)
        .anchor(AnchorEdges::bottom_bar())
        .exclusive_zone(64)
    .build()?;
```

Callbacks are registered per-surface:

```rust
shell.on("TopBar", "workspace_clicked", |control| {
    // Handle TopBar events
    Value::Void
})?;

shell.on("Dock", "app_clicked", |control| {
    // Handle Dock events
    Value::Void
})?;
```

Both surfaces run in the same event loop and share the same Slint compilation result.
