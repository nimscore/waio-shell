use crate::wayland::surfaces::app_state::AppState;
use crate::wayland::surfaces::display_metrics::DisplayMetrics;
use log::info;
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::ZwlrLayerShellV1,
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    globals::GlobalListContents,
    protocol::{
        wl_compositor::WlCompositor,
        wl_output::{self, WlOutput},
        wl_pointer::{self, WlPointer},
        wl_registry::WlRegistry,
        wl_seat::WlSeat,
        wl_surface::WlSurface,
    },
};
use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
    wp_fractional_scale_v1::{self, WpFractionalScaleV1},
};
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};
use wayland_protocols::xdg::shell::client::{
    xdg_popup::{self, XdgPopup},
    xdg_positioner::XdgPositioner,
    xdg_surface::{self, XdgSurface},
    xdg_wm_base::{self, XdgWmBase},
};

impl Dispatch<ZwlrLayerSurfaceV1, ()> for AppState {
    fn event(
        state: &mut Self,
        layer_surface: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _data: &(),
        _conn: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                let layer_surface_id = layer_surface.id();
                let Some(window) = state.get_output_by_layer_surface_mut(&layer_surface_id) else {
                    info!(
                        "Could not find window for layer surface {:?}",
                        layer_surface_id
                    );
                    return;
                };

                window.handle_layer_surface_configure(layer_surface, serial, width, height);
            }
            zwlr_layer_surface_v1::Event::Closed => {
                let layer_surface_id = layer_surface.id();
                if let Some(window) = state.get_output_by_layer_surface_mut(&layer_surface_id) {
                    window.handle_layer_surface_closed();
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, ()> for AppState {
    fn event(
        state: &mut Self,
        proxy: &WlOutput,
        event: <WlOutput as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_output::Event::Mode { width, height, .. } => {
                let output_id = proxy.id();
                if let Some(window) = state.get_output_by_output_id_mut(&output_id) {
                    window.handle_output_mode(width, height);
                }
            }
            wl_output::Event::Description { ref description } => {
                info!("WlOutput description: {description:?}");
            }
            wl_output::Event::Scale { ref factor } => {
                info!("WlOutput factor scale: {factor:?}");
            }
            wl_output::Event::Name { ref name } => {
                info!("WlOutput name: {name:?}");
            }
            wl_output::Event::Geometry {
                x,
                y,
                physical_width,
                physical_height,
                subpixel,
                make,
                model,
                transform,
            } => {
                info!(
                    "WlOutput geometry: x={x}, y={y}, physical_width={physical_width}, physical_height={physical_height}, subpixel={subpixel:?}, make={make:?}, model={model:?}, transform={transform:?}"
                );
            }
            wl_output::Event::Done => {
                info!("WlOutput done");
            }
            _ => {}
        }
    }
}

impl Dispatch<WlPointer, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &WlPointer,
        event: <WlPointer as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                serial,
                surface,
                surface_x,
                surface_y,
            } => {
                info!("Pointer entered surface {:?}", surface.id());

                let surface_id = surface.id();

                if let Some(window) = state.get_output_by_surface_mut(&surface_id) {
                    window.handle_pointer_enter(serial, &surface, surface_x, surface_y);

                    if let Some(key) = state.get_key_by_surface(&surface_id).cloned() {
                        state.set_active_output(Some(key));
                    }
                } else {
                    let key = state.get_key_by_popup(&surface_id);
                    if let Some(window) = state.find_output_by_popup_mut(&surface_id) {
                        window.handle_pointer_enter(serial, &surface, surface_x, surface_y);

                        if let Some(key) = key {
                            state.set_active_output(Some(key));
                        }
                    }
                }
            }

            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                if let Some(output_key) = state.active_output().cloned() {
                    if let Some(window) = state.get_output_by_key_mut(&output_key) {
                        window.handle_pointer_motion(surface_x, surface_y);
                    }
                }
            }

            wl_pointer::Event::Leave { .. } => {
                if let Some(output_key) = state.active_output().cloned() {
                    if let Some(window) = state.get_output_by_key_mut(&output_key) {
                        window.handle_pointer_leave();
                    }
                }
                state.set_active_output(None);
            }

            wl_pointer::Event::Button {
                serial,
                state: button_state,
                ..
            } => {
                if let Some(output_key) = state.active_output().cloned() {
                    if let Some(window) = state.get_output_by_key_mut(&output_key) {
                        window.handle_pointer_button(serial, button_state);
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WpFractionalScaleV1, ()> for AppState {
    fn event(
        state: &mut Self,
        proxy: &WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            let scale_float = DisplayMetrics::scale_factor_from_120ths(scale);
            info!("Fractional scale received: {scale_float} ({scale}x)");

            for window in state.all_outputs_mut() {
                window.handle_fractional_scale(proxy, scale);
            }
        }
    }
}

impl Dispatch<XdgWmBase, ()> for AppState {
    fn event(
        _state: &mut Self,
        xdg_wm_base: &XdgWmBase,
        event: xdg_wm_base::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            use crate::wayland::surfaces::surface_state::WindowState;
            WindowState::handle_xdg_wm_base_ping(xdg_wm_base, serial);
        }
    }
}

impl Dispatch<XdgPopup, ()> for AppState {
    fn event(
        state: &mut Self,
        xdg_popup: &XdgPopup,
        event: xdg_popup::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            xdg_popup::Event::Configure {
                x,
                y,
                width,
                height,
            } => {
                let popup_id = xdg_popup.id();
                for window in state.all_outputs_mut() {
                    if let Some(popup_manager) = window.popup_manager() {
                        if popup_manager.find_by_xdg_popup(&popup_id).is_some() {
                            window.handle_xdg_popup_configure(xdg_popup, x, y, width, height);
                            break;
                        }
                    }
                }
            }
            xdg_popup::Event::PopupDone => {
                info!("XdgPopup dismissed by compositor");
                let popup_id = xdg_popup.id();

                for window in state.all_outputs_mut() {
                    let popup_handle = window
                        .popup_manager()
                        .as_ref()
                        .and_then(|pm| pm.find_by_xdg_popup(&popup_id));

                    if popup_handle.is_some() {
                        window.handle_xdg_popup_done(xdg_popup);
                        break;
                    }
                }
            }
            xdg_popup::Event::Repositioned { token } => {
                info!("XdgPopup repositioned with token {token}");
            }
            _ => {}
        }
    }
}

impl Dispatch<XdgSurface, ()> for AppState {
    fn event(
        state: &mut Self,
        xdg_surface: &XdgSurface,
        event: xdg_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            let xdg_surface_id = xdg_surface.id();
            for window in state.all_outputs_mut() {
                if let Some(popup_manager) = window.popup_manager() {
                    if popup_manager.find_by_xdg_surface(&xdg_surface_id).is_some() {
                        window.handle_xdg_surface_configure(xdg_surface, serial);
                        break;
                    }
                }
            }
        }
    }
}

macro_rules! impl_empty_dispatch_app {
    ($(($t:ty, $u:ty)),+) => {
        $(
            impl Dispatch<$t, $u> for AppState {
                fn event(
                    _state: &mut Self,
                    _proxy: &$t,
                    _event: <$t as wayland_client::Proxy>::Event,
                    _data: &$u,
                    _conn: &Connection,
                    _qhandle: &QueueHandle<Self>,
                ) {
                  info!("Implement empty dispatch event for {:?}", stringify!($t));
                }
            }
        )+
    };
}

impl_empty_dispatch_app!(
    (WlRegistry, GlobalListContents),
    (WlCompositor, ()),
    (WlSurface, ()),
    (ZwlrLayerShellV1, ()),
    (WlSeat, ()),
    (WpFractionalScaleManagerV1, ()),
    (WpViewporter, ()),
    (WpViewport, ()),
    (XdgPositioner, ())
);
