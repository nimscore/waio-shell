use crate::impl_empty_dispatch;
use crate::wayland::surfaces::surface_state::WindowState;
use log::info;
use slint::{
    PhysicalSize,
    platform::{PointerEventButton, WindowEvent},
};
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

impl Dispatch<ZwlrLayerSurfaceV1, ()> for WindowState {
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

                let output_width = state.output_size().width;
                let scale_factor = state.scale_factor();

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
                    let h = state.height();
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
                    state.output_size().height
                );

                state.update_size(clamped_width, target_height);
            }
            zwlr_layer_surface_v1::Event::Closed => {
                info!("Layer surface closed");
            }
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, ()> for WindowState {
    fn event(
        state: &mut Self,
        _proxy: &WlOutput,
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
                state.set_output_size(PhysicalSize::new(width, height));
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

impl Dispatch<WlPointer, ()> for WindowState {
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
                state.set_last_pointer_serial(serial);
                state.set_current_pointer_position(surface_x, surface_y);

                state.find_window_for_surface(&surface);
                let position = state.current_pointer_position();

                state.dispatch_to_active_window(WindowEvent::PointerMoved { position });
            }

            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                state.set_current_pointer_position(surface_x, surface_y);
                let position = state.current_pointer_position();

                state.dispatch_to_active_window(WindowEvent::PointerMoved { position });
            }

            wl_pointer::Event::Leave { .. } => {
                state.dispatch_to_active_window(WindowEvent::PointerExited);
                state.clear_active_window();
            }

            wl_pointer::Event::Button {
                serial,
                state: button_state,
                ..
            } => {
                state.set_last_pointer_serial(serial);
                let position = state.current_pointer_position();
                let event = match button_state {
                    WEnum::Value(wl_pointer::ButtonState::Pressed) => WindowEvent::PointerPressed {
                        button: PointerEventButton::Left,
                        position,
                    },
                    _ => WindowEvent::PointerReleased {
                        button: PointerEventButton::Left,
                        position,
                    },
                };

                state.dispatch_to_active_window(event);
            }
            _ => {}
        }
    }
}

impl Dispatch<WpFractionalScaleV1, ()> for WindowState {
    fn event(
        state: &mut Self,
        proxy: &WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            #[allow(clippy::cast_precision_loss)]
            let scale_float = scale as f32 / 120.0;
            info!("Fractional scale received: {scale_float} ({scale}x)");
            state.update_scale_for_fractional_scale_object(proxy, scale);
        }
    }
}

impl Dispatch<XdgWmBase, ()> for WindowState {
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

impl Dispatch<XdgPopup, ()> for WindowState {
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

                if let Some(popup_service) = state.popup_service() {
                    let popup_id = xdg_popup.id();
                    if let Some(handle) = popup_service.find_by_xdg_popup(&popup_id) {
                        info!(
                            "Marking popup with handle {handle:?} as configured after XdgPopup::Configure"
                        );
                        popup_service.mark_popup_configured(handle);
                        popup_service.manager().mark_all_popups_dirty();
                    }
                }
            }
            xdg_popup::Event::PopupDone => {
                info!("XdgPopup dismissed by compositor");
                let popup_id = xdg_popup.id();
                let popup_handle = state
                    .popup_service()
                    .as_ref()
                    .and_then(|ps| ps.find_by_xdg_popup(&popup_id));

                if let Some(handle) = popup_handle {
                    info!("Destroying popup with handle {handle:?}");
                    state.clear_active_window_if_popup(handle.key());
                    if let Some(popup_service) = state.popup_service() {
                        let _result = popup_service.close(handle);
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

impl Dispatch<XdgSurface, ()> for WindowState {
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

            if let Some(popup_service) = state.popup_service() {
                info!("Marking all popups as dirty after Configure");
                popup_service.manager().mark_all_popups_dirty();
            }
        }
    }
}

impl_empty_dispatch!(
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
