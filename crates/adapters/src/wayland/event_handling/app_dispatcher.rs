use crate::wayland::surfaces::app_state::AppState;
use crate::wayland::surfaces::display_metrics::DisplayMetrics;
use log::info;
use slint::PhysicalSize;
use slint::platform::{PointerEventButton, WindowEvent};
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::ZwlrLayerShellV1,
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};
use wayland_client::WEnum;
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
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_precision_loss)]
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
                info!("Layer surface configured with compositor size: {width}x{height}");
                layer_surface.ack_configure(serial);

                let layer_surface_id = layer_surface.id();
                let Some(window) = state.get_output_by_layer_surface_mut(&layer_surface_id) else {
                    info!(
                        "Could not find window for layer surface {:?}",
                        layer_surface_id
                    );
                    return;
                };

                let output_width = window.output_size().width;
                let scale_factor = window.scale_factor();

                let target_width = if width == 0 || (width == 1 && output_width > 1) {
                    if scale_factor > 1.0 {
                        (output_width as f32 / scale_factor).round() as u32
                    } else {
                        output_width
                    }
                } else {
                    width
                };

                let target_height = if height > 0 {
                    height
                } else {
                    let h = window.height();
                    if scale_factor > 1.0 {
                        (h as f32 / scale_factor).round() as u32
                    } else {
                        h
                    }
                };

                let clamped_width = target_width.min(output_width);

                info!(
                    "Using dimensions: {}x{} (clamped from {}x{}, output: {}x{})",
                    clamped_width,
                    target_height,
                    target_width,
                    target_height,
                    output_width,
                    window.output_size().height
                );

                window.update_size(clamped_width, target_height);
            }
            zwlr_layer_surface_v1::Event::Closed => {
                info!("Layer surface closed");
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
                info!("WlOutput size changed to {width}x{height}");
                let width = width.try_into().unwrap_or_default();
                let height = height.try_into().unwrap_or_default();

                let output_id = proxy.id();
                if let Some(window) = state.get_output_by_output_id_mut(&output_id) {
                    window.set_output_size(PhysicalSize::new(width, height));
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
                    window.set_last_pointer_serial(serial);
                    window.set_current_pointer_position(surface_x, surface_y);
                    window.set_entered_surface(&surface);
                    let position = window.current_pointer_position();
                    window.dispatch_to_active_window(WindowEvent::PointerMoved { position });

                    if let Some(key) = state.get_key_by_surface(&surface_id).cloned() {
                        state.set_active_output(Some(key));
                    }
                } else {
                    let key = state.get_key_by_popup(&surface_id);
                    if let Some(window) = state.find_output_by_popup_mut(&surface_id) {
                        window.set_last_pointer_serial(serial);
                        window.set_current_pointer_position(surface_x, surface_y);
                        window.set_entered_surface(&surface);
                        let position = window.current_pointer_position();
                        window.dispatch_to_active_window(WindowEvent::PointerMoved { position });

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
                        window.set_current_pointer_position(surface_x, surface_y);
                        let position = window.current_pointer_position();
                        window.dispatch_to_active_window(WindowEvent::PointerMoved { position });
                    }
                }
            }

            wl_pointer::Event::Leave { .. } => {
                if let Some(output_key) = state.active_output().cloned() {
                    if let Some(window) = state.get_output_by_key_mut(&output_key) {
                        window.dispatch_to_active_window(WindowEvent::PointerExited);
                        window.clear_entered_surface();
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
                        window.set_last_pointer_serial(serial);
                        let position = window.current_pointer_position();
                        let event = match button_state {
                            WEnum::Value(wl_pointer::ButtonState::Pressed) => {
                                WindowEvent::PointerPressed {
                                    button: PointerEventButton::Left,
                                    position,
                                }
                            }
                            _ => WindowEvent::PointerReleased {
                                button: PointerEventButton::Left,
                                position,
                            },
                        };
                        window.dispatch_to_active_window(event);
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
                window.update_scale_for_fractional_scale_object(proxy, scale);
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
            info!("XdgWmBase ping received, sending pong with serial {serial}");
            xdg_wm_base.pong(serial);
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
                info!("XdgPopup Configure: position=({x}, {y}), size=({width}x{height})");

                let popup_id = xdg_popup.id();
                for window in state.all_outputs_mut() {
                    if let Some(popup_manager) = window.popup_manager() {
                        if let Some(handle) = popup_manager.find_by_xdg_popup(&popup_id) {
                            info!(
                                "Marking popup with handle {handle:?} as configured after XdgPopup::Configure"
                            );
                            popup_manager.mark_popup_configured(handle.key());
                            popup_manager.mark_all_popups_dirty();
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

                    if let Some(handle) = popup_handle {
                        info!("Destroying popup with handle {handle:?}");
                        if let Some(popup_manager) = window.popup_manager() {
                            let _result = popup_manager.close(handle);
                        }
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
            info!("XdgSurface Configure received, sending ack with serial {serial}");
            xdg_surface.ack_configure(serial);

            let xdg_surface_id = xdg_surface.id();
            for window in state.all_outputs_mut() {
                if let Some(popup_manager) = window.popup_manager() {
                    if popup_manager.find_by_xdg_surface(&xdg_surface_id).is_some() {
                        info!("Marking all popups as dirty after Configure");
                        popup_manager.mark_all_popups_dirty();
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
