# Declarative Config Example

This example demonstrates the declarative configuration approach using `ShellConfig` instead of the fluent builder API.

## What it demonstrates

- Creating a shell from declarative configuration
- Using `ShellConfig` and `SurfaceComponentConfig`
- Specifying all surface properties explicitly via `SurfaceConfig`
- Separating configuration building from shell creation
- Loading UI from file path via `CompiledUiSource::file()`

## Running

```bash
cd examples/declarative-config
RUST_LOG=info cargo run
```

## When to use declarative config

- Loading configuration from external sources
- Programmatically generating configurations
- When you need full control over all configuration fields
- Building configuration tools or editors

For simple use cases, the fluent builder API is more ergonomic.
