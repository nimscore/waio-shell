# Runtime Surface Config Example

This example demonstrates layer-shika's runtime surface configuration capabilities using the Surface Control API.

## Features Demonstrated

1. **Dynamic Sizing**: Toggle between compact (32px) and expanded (64px) bar heights
2. **Anchor Position Control**: Switch between top and bottom screen edges at runtime
3. **Layer Management**: Cycle through Background, Bottom, Top, and Overlay layers
4. **Channel-based UI Updates**: Use event loop channels to update UI state from callbacks
5. **Surface Control API**: Manipulate surfaces via callback handlers

## Controls

- **Expand/Collapse Button**: Toggle between 32px and 64px bar heights
- **Switch Anchor**: Toggle between top and bottom screen positions
- **Switch Layer**: Cycle through Background → Bottom → Top → Overlay layers

## Running the Example

```bash
cargo run -p runtime-surface-config
```

## Implementation Highlights

### Control from Slint Callbacks

```rust
shell.on("Bar", "toggle-size", move |control| {
    let bar = control.surface("Bar");
    bar.resize(width, height)?;
    bar.set_exclusive_zone(new_size)?;
    Value::Struct(Struct::from_iter([("expanded".into(), is_expanded.into())]))
})?;
```

### Channel-based UI Updates

```rust
let (_token, sender) = handle.add_channel(|message: UiUpdate, app_state| {
    for surface in app_state.all_outputs() {
        let component = surface.component_instance();
        match &message {
            UiUpdate::IsExpanded(is_expanded) => {
                component.set_property("is-expanded", (*is_expanded).into())?;
            }
            // ... other updates
        }
    }
})?;
```

## API Patterns

This example showcases the Surface Control API pattern:

**SurfaceControlHandle** (channel-based):

- Accessible via `control.surface(name)` in callback handlers
- Safe to call from Slint callbacks
- Commands execute asynchronously in event loop

**Event Loop Channels**:

- Use `add_channel` to create message handlers
- Send messages from callbacks to update UI state
- Process messages in event loop context
- Access all surfaces via `app_state.all_outputs()`
