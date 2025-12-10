use layer_shika::calloop::channel::Sender;
use layer_shika::prelude::*;
use layer_shika::slint::SharedString;
use layer_shika::slint_interpreter::{Struct, Value};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug)]
enum UiUpdate {
    IsExpanded(bool),
    CurrentAnchor(String),
    CurrentLayer(String),
}
enum AnchorPosition {
    Top,
    Bottom,
}

struct BarState {
    is_expanded: bool,
    current_anchor: AnchorPosition,
    current_layer: Layer,
}

impl BarState {
    fn new() -> Self {
        Self {
            is_expanded: false,
            current_anchor: AnchorPosition::Top,
            current_layer: Layer::Top,
        }
    }

    fn anchor_name(&self) -> &'static str {
        match self.current_anchor {
            AnchorPosition::Top => "Top",
            AnchorPosition::Bottom => "Bottom",
        }
    }

    fn next_anchor(&mut self) {
        self.current_anchor = match self.current_anchor {
            AnchorPosition::Top => AnchorPosition::Bottom,
            AnchorPosition::Bottom => AnchorPosition::Top,
        };
    }

    fn get_anchor_edges(&self) -> AnchorEdges {
        match self.current_anchor {
            AnchorPosition::Top => AnchorEdges::top_bar(),
            AnchorPosition::Bottom => AnchorEdges::bottom_bar(),
        }
    }

    fn layer_name(&self) -> &'static str {
        match self.current_layer {
            Layer::Background => "Background",
            Layer::Bottom => "Bottom",
            Layer::Top => "Top",
            Layer::Overlay => "Overlay",
        }
    }

    fn next_layer(&mut self) -> Layer {
        self.current_layer = match self.current_layer {
            Layer::Background => Layer::Bottom,
            Layer::Bottom => Layer::Top,
            Layer::Top => Layer::Overlay,
            Layer::Overlay => Layer::Background,
        };
        self.current_layer
    }
}

fn setup_toggle_size_callback(
    sender: &Rc<Sender<UiUpdate>>,
    shell: &Shell,
    state: &Rc<RefCell<BarState>>,
) -> Result<()> {
    let state_clone = Rc::clone(state);
    let sender_clone = Rc::clone(sender);
    shell.on("Bar", "toggle-size", move |control| {
        let is_expanded = {
            let mut st = state_clone.borrow_mut();
            st.is_expanded = !st.is_expanded;

            let new_size = if st.is_expanded { 64 } else { 32 };

            let (width, height) = match st.current_anchor {
                AnchorPosition::Top | AnchorPosition::Bottom => {
                    log::info!("Resizing horizontal bar to {}px", new_size);
                    (0, new_size)
                }
            };

            let bar = control.surface("Bar");
            if let Err(e) = bar.resize(width, height) {
                log::error!("Failed to resize bar: {}", e);
            }

            if let Err(e) = bar.set_exclusive_zone(new_size.try_into().unwrap_or(32)) {
                log::error!("Failed to set exclusive zone: {}", e);
            }

            if let Err(e) = control.surface("Bar").set_margins((0, 0, 0, 0)) {
                log::error!("Failed to set margins: {}", e);
            }

            log::info!(
                "Updated bar state: size={}, is_expanded={}",
                new_size,
                st.is_expanded
            );

            st.is_expanded
        };

        if let Err(e) = sender_clone.send(UiUpdate::IsExpanded(is_expanded)) {
            log::error!("Failed to send UI update: {}", e);
        }

        Value::Struct(Struct::from_iter([("expanded".into(), is_expanded.into())]))
    })
}

fn setup_anchor_switch_callback(
    sender: &Rc<Sender<UiUpdate>>,
    shell: &Shell,
    state: &Rc<RefCell<BarState>>,
) -> Result<()> {
    let state_clone = Rc::clone(state);
    let sender_clone = Rc::clone(sender);
    shell.on("Bar", "switch-anchor", move |control| {
        let anchor_name = {
            let mut st = state_clone.borrow_mut();
            st.next_anchor();

            log::info!("Switching to anchor: {}", st.anchor_name());

            let bar = control.surface("Bar");
            if let Err(e) = bar.set_anchor(st.get_anchor_edges()) {
                log::error!("Failed to apply config: {}", e);
            }

            st.anchor_name()
        };

        if let Err(e) = sender_clone.send(UiUpdate::CurrentAnchor(anchor_name.to_string())) {
            log::error!("Failed to send UI update: {}", e);
        }

        log::info!("Switched to {} anchor", anchor_name);

        Value::Struct(Struct::from_iter([(
            "anchor".into(),
            SharedString::from(anchor_name).into(),
        )]))
    })
}

fn setup_layer_switch_callback(
    sender: &Rc<Sender<UiUpdate>>,
    shell: &Shell,
    state: &Rc<RefCell<BarState>>,
) -> Result<()> {
    let state_clone = Rc::clone(state);
    let sender_clone = Rc::clone(sender);
    shell.on("Bar", "switch-layer", move |control| {
        let layer_name = {
            let mut st = state_clone.borrow_mut();
            let new_layer = st.next_layer();

            log::info!("Switching to layer: {:?}", new_layer);

            let bar = control.surface("Bar");
            if let Err(e) = bar.set_layer(new_layer) {
                log::error!("Failed to set layer: {}", e);
            }

            st.layer_name()
        };

        if let Err(e) = sender_clone.send(UiUpdate::CurrentLayer(layer_name.to_string())) {
            log::error!("Failed to send UI update: {}", e);
        }

        log::info!("Switched to {} layer", layer_name);

        Value::Struct(Struct::from_iter([(
            "layer".into(),
            SharedString::from(layer_name).into(),
        )]))
    })
}

fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    log::info!("Starting runtime-control example");
    log::info!("This example demonstrates dynamic surface manipulation at runtime");

    let ui_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/bar.slint");

    let state = Rc::new(RefCell::new(BarState::new()));

    let mut shell = Shell::from_file(ui_path)
        .surface("Bar")
        .height(32)
        .anchor(AnchorEdges::top_bar())
        .exclusive_zone(32)
        .namespace("runtime-control-example")
        .build()?;

    shell.with_all_surfaces(|_name, component| {
        log::info!("Initializing properties for Bar surface");
        let state_ref = state.borrow();

        let set_property = |name: &str, value: Value| {
            if let Err(e) = component.set_property(name, value) {
                log::error!("Failed to set initial {}: {}", name, e);
            }
        };

        set_property("is-expanded", state_ref.is_expanded.into());
        set_property(
            "current-anchor",
            SharedString::from(state_ref.anchor_name()).into(),
        );
        set_property(
            "current-layer",
            SharedString::from(state_ref.layer_name()).into(),
        );

        log::info!("Initialized properties for Bar surface");
    });

    let handle = shell.event_loop_handle();
    let (_token, sender) = handle.add_channel(|message: UiUpdate, app_state| {
        log::info!("Received UI update: {:?}", message);

        for surface in app_state.all_outputs() {
            let component = surface.component_instance();

            match &message {
                UiUpdate::IsExpanded(is_expanded) => {
                    if let Err(e) = component.set_property("is-expanded", (*is_expanded).into()) {
                        log::error!("Failed to set is-expanded: {}", e);
                    }
                }
                UiUpdate::CurrentAnchor(anchor) => {
                    if let Err(e) = component
                        .set_property("current-anchor", SharedString::from(anchor.as_str()).into())
                    {
                        log::error!("Failed to set current-anchor: {}", e);
                    }
                }
                UiUpdate::CurrentLayer(layer) => {
                    if let Err(e) = component
                        .set_property("current-layer", SharedString::from(layer.as_str()).into())
                    {
                        log::error!("Failed to set current-layer: {}", e);
                    }
                }
            }
        }
    })?;

    let sender_rc = Rc::new(sender);

    setup_toggle_size_callback(&sender_rc, &shell, &state)?;
    setup_anchor_switch_callback(&sender_rc, &shell, &state)?;
    setup_layer_switch_callback(&sender_rc, &shell, &state)?;
    shell.run()?;

    Ok(())
}
