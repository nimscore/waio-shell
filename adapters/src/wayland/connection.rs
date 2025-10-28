use crate::errors::{LayerShikaError, Result};
use std::rc::Rc;
use wayland_client::{Connection, EventQueue};

pub fn initialize_wayland<S>() -> Result<(Rc<Connection>, EventQueue<S>)> {
    let connection =
        Rc::new(Connection::connect_to_env().map_err(LayerShikaError::WaylandConnection)?);
    let event_queue = connection.new_event_queue();
    Ok((connection, event_queue))
}
