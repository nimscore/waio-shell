use log::info;
use slint::{LogicalPosition, PhysicalSize};
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;
use std::rc::Rc;
use wayland_client::{
    protocol::{wl_compositor::WlCompositor, wl_seat::WlSeat, wl_surface::WlSurface},
    QueueHandle,
};
use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
    wp_fractional_scale_v1::WpFractionalScaleV1,
};
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};
use wayland_protocols::xdg::shell::client::{
    xdg_popup::XdgPopup,
    xdg_positioner::{Anchor, ConstraintAdjustment, Gravity, XdgPositioner},
    xdg_surface::XdgSurface,
    xdg_wm_base::XdgWmBase,
};

use super::surface_state::WindowState;

#[allow(dead_code)]
pub struct PopupSurfaceParams<'a> {
    pub compositor: &'a WlCompositor,
    pub xdg_wm_base: &'a XdgWmBase,
    pub parent_layer_surface: &'a ZwlrLayerSurfaceV1,
    pub fractional_scale_manager: Option<&'a WpFractionalScaleManagerV1>,
    pub viewporter: Option<&'a WpViewporter>,
    pub queue_handle: &'a QueueHandle<WindowState>,
    pub position: LogicalPosition,
    pub size: PhysicalSize,
    pub scale_factor: f32,
}

#[allow(dead_code)]
pub struct PopupSurface {
    pub surface: Rc<WlSurface>,
    pub xdg_surface: Rc<XdgSurface>,
    pub xdg_popup: Rc<XdgPopup>,
    pub fractional_scale: Option<Rc<WpFractionalScaleV1>>,
    pub viewport: Option<Rc<WpViewport>>,
}

#[allow(dead_code)]
impl PopupSurface {
    pub fn create(params: &PopupSurfaceParams<'_>) -> Self {
        let surface = Rc::new(params.compositor.create_surface(params.queue_handle, ()));

        let xdg_surface = Rc::new(params.xdg_wm_base.get_xdg_surface(
            &surface,
            params.queue_handle,
            (),
        ));

        let positioner = Self::create_positioner(params);

        let xdg_popup = Rc::new(xdg_surface.get_popup(None, &positioner, params.queue_handle, ()));

        info!("Attaching popup to layer surface via get_popup");
        params.parent_layer_surface.get_popup(&xdg_popup);

        let fractional_scale = params.fractional_scale_manager.map(|manager| {
            info!("Creating fractional scale object for popup surface");
            Rc::new(manager.get_fractional_scale(&surface, params.queue_handle, ()))
        });

        let viewport = params.viewporter.map(|vp| {
            info!("Creating viewport for popup surface");
            Rc::new(vp.get_viewport(&surface, params.queue_handle, ()))
        });

        #[allow(clippy::cast_possible_wrap)]
        #[allow(clippy::cast_precision_loss)]
        #[allow(clippy::cast_possible_truncation)]
        if let Some(ref vp) = viewport {
            let logical_width = (params.size.width as f32 / params.scale_factor) as i32;
            let logical_height = (params.size.height as f32 / params.scale_factor) as i32;
            info!(
                "Setting viewport destination to logical size: {}x{} (physical: {}x{}, scale: {})",
                logical_width,
                logical_height,
                params.size.width,
                params.size.height,
                params.scale_factor
            );
            vp.set_destination(logical_width, logical_height);
        }

        surface.set_buffer_scale(1);
        surface.commit();

        Self {
            surface,
            xdg_surface,
            xdg_popup,
            fractional_scale,
            viewport,
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_possible_wrap)]
    fn create_positioner(params: &PopupSurfaceParams<'_>) -> XdgPositioner {
        let positioner = params
            .xdg_wm_base
            .create_positioner(params.queue_handle, ());

        let x = (params.position.x * params.scale_factor) as i32;
        let y = (params.position.y * params.scale_factor) as i32;
        let width = params.size.width as i32;
        let height = params.size.height as i32;

        positioner.set_anchor_rect(x, y, 1, 1);
        positioner.set_size(width, height);
        positioner.set_anchor(Anchor::TopLeft);
        positioner.set_gravity(Gravity::BottomRight);
        positioner.set_constraint_adjustment(
            ConstraintAdjustment::SlideX
                | ConstraintAdjustment::SlideY
                | ConstraintAdjustment::FlipX
                | ConstraintAdjustment::FlipY
                | ConstraintAdjustment::ResizeX
                | ConstraintAdjustment::ResizeY,
        );

        positioner
    }

    pub fn grab(&self, seat: &WlSeat, serial: u32) {
        info!("Grabbing popup with serial {serial}");
        self.xdg_popup.grab(seat, serial);
    }

    pub fn destroy(&self) {
        info!("Destroying popup surface");
        self.xdg_popup.destroy();
        self.xdg_surface.destroy();
        self.surface.destroy();
    }
}
