use std::cell::Cell;
use std::fs::File;
use std::io::{BufReader, Write};
use std::os::unix::io::{AsFd, BorrowedFd, FromRawFd, IntoRawFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use layer_shika::calloop::{Interest, Mode, TimeoutAction};
use layer_shika::prelude::*;
use layer_shika::slint_interpreter::Value;

struct ReadablePipe {
    reader: BufReader<File>,
}

impl AsFd for ReadablePipe {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.reader.get_ref().as_fd()
    }
}

fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("Starting custom event source example");

    let ui_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/demo.slint");

    let mut shell = Shell::from_file(&ui_path)
        .surface("Main")
        .size(400, 200)
        .layer(Layer::Top)
        .namespace("custom-source-example")
        .build()?;

    let (mut write_end, read_end) = create_pipe()?;

    let readable = ReadablePipe {
        reader: BufReader::new(read_end),
    };

    let handle = shell.event_loop_handle();

    let counter = Cell::new(0i32);

    handle.add_fd(readable, Interest::READ, Mode::Level, move |app_state| {
        log::debug!("Pipe readable event triggered");

        let count = counter.get() + 1;
        counter.set(count);

        let status_text = format!("Events received: {count}");

        for surface in app_state.all_outputs() {
            let component = surface.component_instance();
            if let Err(e) = component.set_property("counter", Value::Number(f64::from(count))) {
                log::error!("Failed to set counter: {e}");
            }
            if let Err(e) =
                component.set_property("status", Value::String(status_text.clone().into()))
            {
                log::error!("Failed to set status: {e}");
            }
        }
    })?;

    thread::spawn(move || {
        let mut event_num = 0;
        loop {
            thread::sleep(Duration::from_millis(500));
            event_num += 1;
            let message = format!("event-{event_num}\n");
            if write_end.write_all(message.as_bytes()).is_err() {
                break;
            }
            if write_end.flush().is_err() {
                break;
            }
            log::debug!("Wrote event {} to pipe", event_num);
        }
    });

    handle.add_timer(Duration::from_secs(1), |_instant, app_state| {
        let time_str = current_time_string();

        for surface in app_state.all_outputs() {
            if let Err(e) = surface
                .component_instance()
                .set_property("time", Value::String(time_str.clone().into()))
            {
                log::error!("Failed to set time property: {e}");
            }
        }

        TimeoutAction::ToInstant(Instant::now() + Duration::from_secs(1))
    })?;

    shell.with_surface("Main", |component| {
        if let Err(e) =
            component.set_property("status", Value::String("Waiting for pipe events...".into()))
        {
            log::error!("Failed to set status: {e}");
        }
    })?;

    shell.run()?;

    Ok(())
}

fn create_pipe() -> Result<(File, File)> {
    let (read_stream, write_stream) = UnixStream::pair()?;

    read_stream.set_nonblocking(true)?;
    write_stream.set_nonblocking(true)?;

    Ok(unsafe {
        (
            FromRawFd::from_raw_fd(write_stream.into_raw_fd()),
            FromRawFd::from_raw_fd(read_stream.into_raw_fd()),
        )
    })
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
