use super::context::EGLContext;
use crate::errors::{EGLError, LayerShikaError, Result};
use glutin::{
    api::egl::{
        config::Config,
        context::{NotCurrentContext, PossiblyCurrentContext},
        display::Display,
        surface::Surface,
    },
    config::ConfigTemplateBuilder,
    context::ContextAttributesBuilder,
    prelude::*,
    surface::{SurfaceAttributesBuilder, WindowSurface},
};
use log::{debug, info};
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use slint::PhysicalSize;
use std::{cell::RefCell, ffi::c_void, num::NonZeroU32, ptr::NonNull, rc::Rc};
use wayland_client::backend::ObjectId;

pub struct RenderContextFactory {
    shared_state: RefCell<Option<SharedRenderState>>,
}

struct SharedRenderState {
    display: Display,
    config: Config,
    primary_context: PossiblyCurrentContext,
}

impl RenderContextFactory {
    #[must_use]
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            shared_state: RefCell::new(None),
        })
    }

    pub fn create_context(
        &self,
        display_id: &ObjectId,
        surface_id: &ObjectId,
        size: PhysicalSize,
    ) -> Result<EGLContext> {
        let mut state = self.shared_state.borrow_mut();

        if state.is_none() {
            info!("Creating primary EGL context (will be shared with subsequent contexts)");
            let new_state = self.create_primary_context(display_id, surface_id, size)?;
            *state = Some(new_state);
        }

        let Some(shared_state) = state.as_ref() else {
            return Err(LayerShikaError::InvalidInput {
                message: "Shared state initialization failed".into(),
            });
        };

        if shared_state.primary_context.is_current() {
            debug!("Creating shared context while primary is current");
            self.create_shared_context_from_current(
                &shared_state.display,
                &shared_state.config,
                &shared_state.primary_context,
                surface_id,
                size,
            )
        } else {
            debug!("Creating shared context (primary not current)");
            self.create_shared_context(
                &shared_state.display,
                &shared_state.config,
                &shared_state.primary_context,
                surface_id,
                size,
            )
        }
    }

    #[allow(clippy::unused_self)]
    fn create_primary_context(
        &self,
        display_id: &ObjectId,
        surface_id: &ObjectId,
        size: PhysicalSize,
    ) -> Result<SharedRenderState> {
        let display_handle = create_wayland_display_handle(display_id)?;
        let display = unsafe { Display::new(display_handle) }
            .map_err(|e| EGLError::DisplayCreation { source: e.into() })?;

        let config_template = ConfigTemplateBuilder::default();
        let config = select_config(&display, config_template)?;

        let context_attributes = ContextAttributesBuilder::default();
        let not_current = create_context(&display, &config, context_attributes)?;

        let surface_handle = create_surface_handle(surface_id)?;
        let surface = create_surface(&display, &config, surface_handle, size)?;

        let primary_context = not_current
            .make_current(&surface)
            .map_err(|e| EGLError::MakeCurrent { source: e.into() })?;

        info!("Primary EGL context created successfully");

        Ok(SharedRenderState {
            display,
            config,
            primary_context,
        })
    }

    #[allow(clippy::unused_self)]
    fn create_shared_context(
        &self,
        display: &Display,
        config: &Config,
        share_context: &PossiblyCurrentContext,
        surface_id: &ObjectId,
        size: PhysicalSize,
    ) -> Result<EGLContext> {
        let context_attributes = ContextAttributesBuilder::default().with_sharing(share_context);

        let not_current =
            unsafe { display.create_context(config, &context_attributes.build(None)) }
                .map_err(|e| EGLError::ContextCreation { source: e.into() })?;

        let surface_handle = create_surface_handle(surface_id)?;
        let surface = create_surface(display, config, surface_handle, size)?;

        let context = not_current
            .make_current(&surface)
            .map_err(|e| EGLError::MakeCurrent { source: e.into() })?;

        info!("Shared EGL context created successfully");

        Ok(EGLContext::from_raw(surface, context))
    }

    #[allow(clippy::unused_self)]
    fn create_shared_context_from_current(
        &self,
        display: &Display,
        config: &Config,
        share_context: &PossiblyCurrentContext,
        surface_id: &ObjectId,
        size: PhysicalSize,
    ) -> Result<EGLContext> {
        let context_attributes = ContextAttributesBuilder::default().with_sharing(share_context);

        let not_current =
            unsafe { display.create_context(config, &context_attributes.build(None)) }
                .map_err(|e| EGLError::ContextCreation { source: e.into() })?;

        let surface_handle = create_surface_handle(surface_id)?;
        let surface = create_surface(display, config, surface_handle, size)?;

        let context = not_current
            .make_current(&surface)
            .map_err(|e| EGLError::MakeCurrent { source: e.into() })?;

        info!("Shared EGL context created successfully (from current)");

        Ok(EGLContext::from_raw(surface, context))
    }
}

impl Default for RenderContextFactory {
    fn default() -> Self {
        Self {
            shared_state: RefCell::new(None),
        }
    }
}

fn create_wayland_display_handle(display_id: &ObjectId) -> Result<RawDisplayHandle> {
    let display = NonNull::new(display_id.as_ptr().cast::<c_void>()).ok_or_else(|| {
        LayerShikaError::InvalidInput {
            message: "Failed to create NonNull pointer for display".into(),
        }
    })?;
    let handle = WaylandDisplayHandle::new(display);
    Ok(RawDisplayHandle::Wayland(handle))
}

fn select_config(
    glutin_display: &Display,
    config_template: ConfigTemplateBuilder,
) -> Result<Config> {
    let mut configs = unsafe { glutin_display.find_configs(config_template.build()) }
        .map_err(|e| EGLError::ConfigSelection { source: e.into() })?;
    configs
        .next()
        .ok_or_else(|| EGLError::NoCompatibleConfig.into())
}

fn create_context(
    glutin_display: &Display,
    config: &Config,
    context_attributes: ContextAttributesBuilder,
) -> Result<NotCurrentContext> {
    unsafe { glutin_display.create_context(config, &context_attributes.build(None)) }
        .map_err(|e| EGLError::ContextCreation { source: e.into() }.into())
}

fn create_surface_handle(surface_id: &ObjectId) -> Result<RawWindowHandle> {
    let surface = NonNull::new(surface_id.as_ptr().cast::<c_void>()).ok_or_else(|| {
        LayerShikaError::InvalidInput {
            message: "Failed to create NonNull pointer for surface".into(),
        }
    })?;
    let handle = WaylandWindowHandle::new(surface);
    Ok(RawWindowHandle::Wayland(handle))
}

fn create_surface(
    glutin_display: &Display,
    config: &Config,
    surface_handle: RawWindowHandle,
    size: PhysicalSize,
) -> Result<Surface<WindowSurface>> {
    let width = NonZeroU32::new(size.width).ok_or_else(|| LayerShikaError::InvalidInput {
        message: "Width cannot be zero".into(),
    })?;

    let height = NonZeroU32::new(size.height).ok_or_else(|| LayerShikaError::InvalidInput {
        message: "Height cannot be zero".into(),
    })?;

    let attrs =
        SurfaceAttributesBuilder::<WindowSurface>::new().build(surface_handle, width, height);

    unsafe { glutin_display.create_window_surface(config, &attrs) }
        .map_err(|e| EGLError::SurfaceCreation { source: e.into() }.into())
}
