# Session Lock Selectors Example

This example demonstrates using **selectors** to configure session lock surfaces with different properties per output. It shows how to set different themes or configurations for lock screens on different monitors.

## Key Features

- Creates both a layer shell surface (status bar) and session lock surfaces
- Uses `select_lock()` to apply configurations to specific lock surfaces
- Demonstrates per-output theming (dark theme on primary output)
- Shows how to handle lock/unlock callbacks with selectors

## Selector Usage

```rust
// Apply to all lock surfaces
shell.select_lock(Surface::all())
    .on_callback_with_args("unlock_requested", handler);

// Apply only to primary output's lock surface
shell.select_lock(Output::Primary)
    .set_property("theme", &Value::from("dark"))?;

// Apply to regular layer surface
shell.select(Surface::named("Main"))
    .on_callback("lock_requested", handler);
```

## Run

```bash
cargo run -p session-lock-selectors
```

## Usage Flow

1. Application starts with a status bar showing a "Lock" button
2. Click the "Lock" button to activate the session lock
3. Lock screens appear on all outputs
4. Primary output's lock screen uses dark theme
5. Enter any password and click "Unlock" to deactivate
6. Application returns to normal mode with status bar

## When to Use This Pattern

Use session lock selectors when you need to:

- Configure lock screens differently per output
- Apply different themes or properties to specific monitors
- Handle callbacks consistently across all lock surfaces
- Combine layer shell surfaces with session locks
