use crate::impl_empty_dispatch;
use log::info;
use slint::{
    platform::{PointerEventButton, WindowEvent},
    PhysicalSize,
};
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::ZwlrLayerShellV1,
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};
use wayland_client::WEnum;
use wayland_client::{
    globals::GlobalListContents,
    protocol::{
        wl_compositor::WlCompositor,
        wl_output::{self, WlOutput},
        wl_pointer::{self, WlPointer},
        wl_registry::WlRegistry,
        wl_seat::WlSeat,
        wl_surface::WlSurface,
    },
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
    wp_fractional_scale_v1::{self, WpFractionalScaleV1},
};
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};

use super::WindowState;

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
                info!(
                    "Layer surface configured with compositor size: {}x{}",
                    width, height
                );
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
                info!("WlOutput size changed to {}x{}", width, height);
                let width = width.try_into().unwrap_or_default();
                let height = height.try_into().unwrap_or_default();
                state.set_output_size(PhysicalSize::new(width, height));
            }
            wl_output::Event::Description { ref description } => {
                info!("WlOutput description: {:?}", description);
            }
            wl_output::Event::Scale { ref factor } => {
                info!("WlOutput factor scale: {:?}", factor);
            }
            wl_output::Event::Name { ref name } => {
                info!("WlOutput name: {:?}", name);
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
                info!("WlOutput geometry: x={}, y={}, physical_width={}, physical_height={}, subpixel={:?}, make={:?}, model={:?}, transform={:?}", x, y, physical_width, physical_height, subpixel, make, model, transform);
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
                surface_x,
                surface_y,
                ..
            }
            | wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                state.set_current_pointer_position(surface_x, surface_y);
                let logical_position = state.current_pointer_position();
                state.window().dispatch_event(WindowEvent::PointerMoved {
                    position: *logical_position,
                });
            }

            wl_pointer::Event::Leave { .. } => {
                state.window().dispatch_event(WindowEvent::PointerExited);
            }

            wl_pointer::Event::Button {
                state: button_state,
                ..
            } => {
                let event = match button_state {
                    WEnum::Value(wl_pointer::ButtonState::Pressed) => WindowEvent::PointerPressed {
                        button: PointerEventButton::Left,
                        position: *state.current_pointer_position(),
                    },
                    _ => WindowEvent::PointerReleased {
                        button: PointerEventButton::Left,
                        position: *state.current_pointer_position(),
                    },
                };
                state.window().dispatch_event(event);
            }
            _ => {}
        }
    }
}

impl Dispatch<WpFractionalScaleV1, ()> for WindowState {
    fn event(
        state: &mut Self,
        _proxy: &WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            #[allow(clippy::cast_precision_loss)]
            let scale_float = scale as f32 / 120.0;
            info!("Fractional scale received: {scale_float} ({scale}x)");
            state.update_scale_factor(scale);
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
    (WpViewport, ())
);
