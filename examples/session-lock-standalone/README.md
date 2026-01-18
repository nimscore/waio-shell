# Session Lock Standalone Example

This example demonstrates creating a **standalone session lock application** without any layer shell surfaces. This is useful for dedicated lock screen applications that don't need persistent UI elements.

## Key Differences from Regular Session Lock

### Regular Session Lock (`session-lock`)
- Creates a layer shell surface (status bar, panel, etc.)
- Requires `wlr-layer-shell` protocol
- Lock is activated via button click or external trigger
- Application has persistent UI even when unlocked

### Standalone Session Lock (this example)
- **No layer shell surfaces** - minimal mode
- Does NOT require `wlr-layer-shell` protocol
- Lock activates immediately on startup
- Application exits when lock is deactivated
- Lighter weight and simpler architecture

## Usage

```bash
cargo run --package session-lock-standalone
```

The lock screen will appear immediately on all outputs. Enter any password and click "Unlock" to deactivate and exit.

## Code Highlights

```rust
// Build shell WITHOUT calling .surface() - this creates minimal mode
let mut shell = Shell::from_file(ui_path).build()?;

// Create and activate the session lock immediately
let lock = Rc::new(shell.create_session_lock("LockScreen")?);
lock.activate()?;

// Shell runs until lock is deactivated
shell.run()?;
```

## When to Use This Pattern

Use standalone session locks for:
- Dedicated lock screen applications
- Screen locker daemons that only show UI when locking
- Simpler lock-only tools without status bars
- Testing and development of lock screens in isolation

Use regular session locks with layer surfaces when:
- You need persistent UI (status bar, panel, dock)
- Lock is one feature among others in your application
- You want to trigger lock activation from UI elements
