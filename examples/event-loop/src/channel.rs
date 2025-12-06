use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use layer_shika::calloop::TimeoutAction;
use layer_shika::calloop::channel::Sender;
use layer_shika::prelude::*;
use layer_shika::slint_interpreter::Value;

enum UiMessage {
    UpdateStatus(String),
    IncrementCounter(i32),
    BackgroundTaskComplete(String),
}

fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("Starting channel example");

    let ui_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/demo.slint");

    let mut shell = Shell::from_file(&ui_path)
        .surface("Main")
        .size(400, 200)
        .layer(Layer::Top)
        .namespace("channel-example")
        .build()?;

    let handle = shell.event_loop_handle();

    let (_token, sender) = handle.add_channel(|message: UiMessage, app_state| {
        for window in app_state.all_outputs() {
            let component = window.component_instance();

            match &message {
                UiMessage::UpdateStatus(status) => {
                    if let Err(e) =
                        component.set_property("status", Value::String(status.clone().into()))
                    {
                        log::error!("Failed to set status: {e}");
                    }
                    log::info!("Status updated: {}", status);
                }
                UiMessage::IncrementCounter(delta) => {
                    if let Ok(Value::Number(current)) = component.get_property("counter") {
                        #[allow(clippy::cast_possible_truncation)]
                        let new_value = current as i32 + delta;
                        if let Err(e) =
                            component.set_property("counter", Value::Number(f64::from(new_value)))
                        {
                            log::error!("Failed to set counter: {e}");
                        }
                        log::debug!("Counter: {}", new_value);
                    }
                }
                UiMessage::BackgroundTaskComplete(result) => {
                    if let Err(e) = component
                        .set_property("status", Value::String(format!("Done: {result}").into()))
                    {
                        log::error!("Failed to set status: {e}");
                    }
                    log::info!("Background task complete: {}", result);
                }
            }
        }
    })?;

    handle.add_timer(Duration::from_secs(1), |_instant, app_state| {
        let time_str = current_time_string();

        for window in app_state.all_outputs() {
            if let Err(e) = window
                .component_instance()
                .set_property("time", Value::String(time_str.clone().into()))
            {
                log::error!("Failed to set time property: {e}");
            }
        }

        TimeoutAction::ToInstant(Instant::now() + Duration::from_secs(1))
    })?;

    spawn_background_worker(sender.clone());
    spawn_counter_worker(sender);

    shell.run()?;

    Ok(())
}

fn spawn_background_worker(sender: Sender<UiMessage>) {
    thread::spawn(move || {
        let tasks = vec![
            ("Loading configuration...", 500),
            ("Connecting to services...", 800),
            ("Fetching data...", 1200),
            ("Processing results...", 600),
        ];

        for (status, delay_ms) in tasks {
            if sender
                .send(UiMessage::UpdateStatus(status.to_string()))
                .is_err()
            {
                return;
            }
            thread::sleep(Duration::from_millis(delay_ms));
        }

        if sender
            .send(UiMessage::BackgroundTaskComplete(
                "All tasks finished".to_string(),
            ))
            .is_err()
        {
            return;
        }

        loop {
            thread::sleep(Duration::from_secs(5));
            if sender
                .send(UiMessage::UpdateStatus(
                    "Heartbeat from background".to_string(),
                ))
                .is_err()
            {
                break;
            }
        }
    });
}

fn spawn_counter_worker(sender: Sender<UiMessage>) {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(50));
            if sender.send(UiMessage::IncrementCounter(1)).is_err() {
                break;
            }
        }
    });
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
