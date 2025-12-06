use std::cell::Cell;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use layer_shika::calloop::TimeoutAction;
use layer_shika::prelude::*;
use layer_shika::slint_interpreter::Value;

fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("Starting timer example");

    let ui_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/demo.slint");

    let mut shell = Shell::from_file(&ui_path)
        .surface("Main")
        .size(400, 200)
        .layer(Layer::Top)
        .namespace("timer-example")
        .build()?;

    let handle = shell.event_loop_handle();

    handle.add_timer(Duration::ZERO, |_instant, app_state| {
        let time_str = current_time_string();

        for surface in app_state.all_outputs() {
            if let Err(e) = surface
                .component_instance()
                .set_property("time", Value::String(time_str.clone().into()))
            {
                log::error!("Failed to set time property: {e}");
            }
        }

        log::debug!("Timer tick: {}", time_str);

        TimeoutAction::ToInstant(Instant::now() + Duration::from_secs(1))
    })?;

    let counter = Cell::new(0i32);
    handle.add_timer(Duration::ZERO, move |_instant, app_state| {
        let count = counter.get() + 1;
        counter.set(count);

        for surface in app_state.all_outputs() {
            if let Err(e) = surface
                .component_instance()
                .set_property("counter", Value::Number(f64::from(count)))
            {
                log::error!("Failed to set counter property: {e}");
            }
        }

        TimeoutAction::ToInstant(Instant::now() + Duration::from_millis(100))
    })?;

    shell.with_surface("Main", |component| {
        if let Err(e) = component.set_property("status", Value::String("Timer running...".into())) {
            log::error!("Failed to set status property: {e}");
        }
    })?;

    shell.run()?;

    Ok(())
}

fn current_time_string() -> String {
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();

    let hours = (secs / 3600) % 24;
    let minutes = (secs / 60) % 60;
    let seconds = secs % 60;

    format!("{hours:02}:{minutes:02}:{seconds:02}")
}
