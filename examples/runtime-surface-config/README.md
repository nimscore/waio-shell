# Runtime Surface Config Example

This example demonstrates waio-shell's runtime surface configuration capabilities using the instance-based Surface Control API.

## Features Demonstrated

1. **Dynamic Sizing**: Toggle between compact (32px) and expanded (64px) bar heights
2. **Anchor Position Control**: Switch between top and bottom screen edges at runtime
3. **Layer Management**: Cycle through Background, Bottom, Top, and Overlay layers
4. **Instance-based Targeting**: Use `CallbackContext` to control the specific surface that invoked the callback
5. **Slint State Management**: State lives in Slint component properties, Rust callbacks apply Wayland configuration

## Controls

- **Expand/Collapse Button**: Toggle between 32px and 64px bar heights
- **Switch Anchor**: Toggle between top and bottom screen positions
- **Switch Layer**: Cycle through Top → Overlay → Bottom → Background layers

## Running the Example

```bash
cargo run -p runtime-surface-config
```

## Implementation Highlights

### Slint-managed State

The Slint component owns all state via properties:

```slint
export component Bar inherits Window {
    in-out property <bool> is-expanded: false;
    in-out property <string> current-anchor: "Top";
    in-out property <string> current-layer: "Top";

    callback toggle-size(bool);
    callback switch-anchor(string);
    callback switch-layer(string);

    Button {
        text: is-expanded ? "Collapse" : "Expand";
        clicked => {
            root.is-expanded = !root.is-expanded;
            toggle-size(root.is-expanded);
        }
    }
}
```

### Instance-based Control from Callbacks

Callbacks receive `CallbackContext` and use `this_instance()` to target only the specific surface:

```rust
shell.select(Surface::named("Bar"))
    .on_callback_with_args("toggle-size", move |args, control| {
        let is_expanded = args.first()
            .and_then(|v| v.clone().try_into().ok())
            .unwrap_or(false);

        let new_size = if is_expanded { 64 } else { 32 };

        control.this_instance()
            .configure()
            .size(0, new_size)
            .exclusive_zone(new_size.try_into().unwrap_or(32))
            .apply()?;

    });
```

## API Patterns

- Send configuration commands via `.configure()` builder or individual methods
- Commands execute asynchronously in the event loop
- Multiple surfaces with the same name can be controlled independently

**State Management**:

- Slint component properties hold all UI state
- Slint logic computes next states (e.g., toggling, cycling)
- Rust callbacks receive new values and apply Wayland configuration
- No Rust-side state synchronization needed
