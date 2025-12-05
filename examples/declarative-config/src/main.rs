use std::path::PathBuf;

use layer_shika::prelude::*;
use layer_shika::slint_interpreter::Value;

fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("Starting declarative-config example");

    let config = build_config();

    log::info!("Creating shell with {} surface(s)", config.surfaces.len());

    let mut shell = Shell::from_config(config)?;

    log::info!("Shell has surfaces: {:?}", shell.surface_names());
    log::info!("Has StatusBar surface: {}", shell.has_surface("StatusBar"));

    shell.on("StatusBar", "settings-clicked", |_control| {
        log::info!("Settings button clicked");
        Value::Void
    })?;

    log::info!("Registered callback for settings-clicked");

    shell.run()?;

    Ok(())
}

fn build_config() -> ShellConfig {
    let ui_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/bar.slint");

    ShellConfig {
        ui_source: CompiledUiSource::file(ui_path),
        surfaces: vec![SurfaceComponentConfig::with_config(
            "StatusBar",
            SurfaceConfig {
                dimensions: SurfaceDimension::new(0, 42),
                anchor: AnchorEdges::top_bar(),
                exclusive_zone: 42,
                layer: Layer::Top,
                margin: Margins::default(),
                namespace: "declarative-config-example".to_string(),
                scale_factor: ScaleFactor::default(),
                keyboard_interactivity: KeyboardInteractivity::OnDemand,
                output_policy: OutputPolicy::PrimaryOnly,
            },
        )],
    }
}
