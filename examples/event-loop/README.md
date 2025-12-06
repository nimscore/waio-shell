# Event Loop Integration Examples

This directory contains examples demonstrating how to integrate custom event sources
with layer-shika's event loop.

## Examples

### Timer (`timer.rs`)

Demonstrates how to add periodic timers to update UI elements (e.g., a clock).

```bash
cargo run --bin timer
```

### Channel (`channel.rs`)

Shows how to use channels for communication between background threads and the UI.
Useful for async operations, network requests, or any off-main-thread work.

```bash
cargo run --bin channel
```

### Custom Event Source (`custom_source.rs`)

Demonstrates adding custom file descriptor-based event sources for I/O monitoring.

```bash
cargo run --bin custom-source
```

## Key Concepts

All examples use `shell.event_loop_handle()` to get a handle that allows registering
event sources with the main event loop. The callbacks receive `&mut AppState` which
provides access to window components and output information.

### Timer Pattern

```rust
let handle = shell.event_loop_handle();
handle.add_timer(Duration::from_secs(1), |_instant, app_state| {
    // Update UI components here
    TimeoutAction::ToInstant(Instant::now() + Duration::from_secs(1))
})?;
```

### Channel Pattern

```rust
let handle = shell.event_loop_handle();
let (_token, sender) = handle.add_channel(|message: MyMessage, app_state| {
    // Handle messages from background threads
})?;

// Send from another thread
std::thread::spawn(move || {
    sender.send(MyMessage::Update("data".into())).unwrap();
});
```

### File Descriptor Pattern

```rust
use layer_shika::calloop::{Generic, Interest, Mode};

let handle = shell.event_loop_handle();
handle.add_fd(file, Interest::READ, Mode::Level, |app_state| {
    // Handle I/O readiness
})?;
```
