use crate::event_loop::FromAppState;
use crate::layer_surface::LayerSurfaceHandle;
use crate::{Error, Result};
use layer_shika_adapters::platform::calloop::channel;
use layer_shika_adapters::platform::slint::ComponentHandle;
use layer_shika_adapters::platform::slint_interpreter::{
    CompilationResult, ComponentDefinition, ComponentInstance, Value,
};
use layer_shika_adapters::{AppState, PopupManager, SurfaceState};
use layer_shika_domain::config::SurfaceConfig;
use layer_shika_domain::entities::output_registry::OutputRegistry;
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::prelude::{
    AnchorEdges, KeyboardInteractivity, Layer, Margins, OutputPolicy, ScaleFactor,
};
use layer_shika_domain::value_objects::dimensions::{PopupDimensions, SurfaceDimension};
use layer_shika_domain::value_objects::output_handle::OutputHandle;
use layer_shika_domain::value_objects::output_info::OutputInfo;
use layer_shika_domain::value_objects::popup_positioning_mode::PopupPositioningMode;
use layer_shika_domain::value_objects::popup_request::{
    PopupHandle, PopupPlacement, PopupRequest, PopupSize,
};
use std::cell::Cell;
use std::rc::Rc;

pub enum PopupCommand {
    Show(PopupRequest),
    Close(PopupHandle),
    Resize {
        handle: PopupHandle,
        width: f32,
        height: f32,
    },
}

pub enum SurfaceCommand {
    Resize {
        name: String,
        width: u32,
        height: u32,
    },
    SetAnchor {
        name: String,
        anchor: AnchorEdges,
    },
    SetExclusiveZone {
        name: String,
        zone: i32,
    },
    SetMargins {
        name: String,
        margins: Margins,
    },
    SetLayer {
        name: String,
        layer: Layer,
    },
    SetOutputPolicy {
        name: String,
        policy: OutputPolicy,
    },
    SetScaleFactor {
        name: String,
        factor: ScaleFactor,
    },
    SetKeyboardInteractivity {
        name: String,
        mode: KeyboardInteractivity,
    },
    ApplyConfig {
        name: String,
        config: SurfaceConfig,
    },
}

pub enum ShellCommand {
    Popup(PopupCommand),
    Surface(SurfaceCommand),
    Render,
}

#[derive(Clone)]
pub struct ShellControl {
    sender: channel::Sender<ShellCommand>,
}

impl ShellControl {
    pub fn new(sender: channel::Sender<ShellCommand>) -> Self {
        Self { sender }
    }

    pub fn show_popup(&self, request: &PopupRequest) -> Result<()> {
        self.sender
            .send(ShellCommand::Popup(PopupCommand::Show(request.clone())))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send popup show command: channel closed".to_string(),
                })
            })
    }

    pub fn show_popup_at_cursor(&self, component: impl Into<String>) -> Result<()> {
        let request = PopupRequest::builder(component.into())
            .placement(PopupPlacement::AtCursor)
            .build();
        self.show_popup(&request)
    }

    pub fn show_popup_centered(&self, component: impl Into<String>) -> Result<()> {
        let request = PopupRequest::builder(component.into())
            .placement(PopupPlacement::AtCursor)
            .mode(PopupPositioningMode::Center)
            .build();
        self.show_popup(&request)
    }

    pub fn show_popup_at_position(
        &self,
        component: impl Into<String>,
        x: f32,
        y: f32,
    ) -> Result<()> {
        let request = PopupRequest::builder(component.into())
            .placement(PopupPlacement::AtPosition { x, y })
            .build();
        self.show_popup(&request)
    }

    pub fn close_popup(&self, handle: PopupHandle) -> Result<()> {
        self.sender
            .send(ShellCommand::Popup(PopupCommand::Close(handle)))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send popup close command: channel closed".to_string(),
                })
            })
    }

    pub fn resize_popup(&self, handle: PopupHandle, width: f32, height: f32) -> Result<()> {
        self.sender
            .send(ShellCommand::Popup(PopupCommand::Resize {
                handle,
                width,
                height,
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send popup resize command: channel closed".to_string(),
                })
            })
    }

    pub fn request_redraw(&self) -> Result<()> {
        self.sender.send(ShellCommand::Render).map_err(|_| {
            Error::Domain(DomainError::Configuration {
                message: "Failed to send redraw command: channel closed".to_string(),
            })
        })
    }

    pub fn surface(&self, name: impl Into<String>) -> SurfaceControlHandle {
        SurfaceControlHandle {
            name: name.into(),
            sender: self.sender.clone(),
        }
    }
}

pub struct SurfaceControlHandle {
    name: String,
    sender: channel::Sender<ShellCommand>,
}

impl SurfaceControlHandle {
    pub fn resize(&self, width: u32, height: u32) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(SurfaceCommand::Resize {
                name: self.name.clone(),
                width,
                height,
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send surface resize command: channel closed".to_string(),
                })
            })
    }

    pub fn set_width(&self, width: u32) -> Result<()> {
        self.resize(width, 0)
    }

    pub fn set_height(&self, height: u32) -> Result<()> {
        self.resize(0, height)
    }

    pub fn set_anchor(&self, anchor: AnchorEdges) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(SurfaceCommand::SetAnchor {
                name: self.name.clone(),
                anchor,
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send surface set_anchor command: channel closed"
                        .to_string(),
                })
            })
    }

    pub fn set_exclusive_zone(&self, zone: i32) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(SurfaceCommand::SetExclusiveZone {
                name: self.name.clone(),
                zone,
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send surface set_exclusive_zone command: channel closed"
                        .to_string(),
                })
            })
    }

    pub fn set_margins(&self, margins: impl Into<Margins>) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(SurfaceCommand::SetMargins {
                name: self.name.clone(),
                margins: margins.into(),
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send surface set_margins command: channel closed"
                        .to_string(),
                })
            })
    }

    pub fn set_layer(&self, layer: Layer) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(SurfaceCommand::SetLayer {
                name: self.name.clone(),
                layer,
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send surface set_layer command: channel closed".to_string(),
                })
            })
    }

    pub fn set_output_policy(&self, policy: OutputPolicy) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(SurfaceCommand::SetOutputPolicy {
                name: self.name.clone(),
                policy,
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send surface set_output_policy command: channel closed"
                        .to_string(),
                })
            })
    }

    pub fn set_scale_factor(&self, factor: ScaleFactor) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(SurfaceCommand::SetScaleFactor {
                name: self.name.clone(),
                factor,
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send surface set_scale_factor command: channel closed"
                        .to_string(),
                })
            })
    }

    pub fn set_keyboard_interactivity(&self, mode: KeyboardInteractivity) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(
                SurfaceCommand::SetKeyboardInteractivity {
                    name: self.name.clone(),
                    mode,
                },
            ))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message:
                        "Failed to send surface set_keyboard_interactivity command: channel closed"
                            .to_string(),
                })
            })
    }

    pub fn apply_config(&self, config: SurfaceConfig) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(SurfaceCommand::ApplyConfig {
                name: self.name.clone(),
                config,
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send surface apply_config command: channel closed"
                        .to_string(),
                })
            })
    }

    pub fn configure(self) -> RuntimeSurfaceConfigBuilder {
        RuntimeSurfaceConfigBuilder {
            handle: self,
            config: SurfaceConfig::new(),
        }
    }
}

pub struct RuntimeSurfaceConfigBuilder {
    handle: SurfaceControlHandle,
    config: SurfaceConfig,
}

impl RuntimeSurfaceConfigBuilder {
    #[must_use]
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.config.dimensions = SurfaceDimension::from_raw(width, height);
        self
    }

    #[must_use]
    pub fn width(mut self, width: u32) -> Self {
        self.config.dimensions = SurfaceDimension::from_raw(width, self.config.dimensions.height());
        self
    }

    #[must_use]
    pub fn height(mut self, height: u32) -> Self {
        self.config.dimensions = SurfaceDimension::from_raw(self.config.dimensions.width(), height);
        self
    }

    #[must_use]
    pub const fn layer(mut self, layer: Layer) -> Self {
        self.config.layer = layer;
        self
    }

    #[must_use]
    pub fn margins(mut self, margins: impl Into<Margins>) -> Self {
        self.config.margin = margins.into();
        self
    }

    #[must_use]
    pub const fn anchor(mut self, anchor: AnchorEdges) -> Self {
        self.config.anchor = anchor;
        self
    }

    #[must_use]
    pub const fn exclusive_zone(mut self, zone: i32) -> Self {
        self.config.exclusive_zone = zone;
        self
    }

    #[must_use]
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.config.namespace = namespace.into();
        self
    }

    #[must_use]
    pub const fn keyboard_interactivity(mut self, mode: KeyboardInteractivity) -> Self {
        self.config.keyboard_interactivity = mode;
        self
    }

    #[must_use]
    pub fn output_policy(mut self, policy: OutputPolicy) -> Self {
        self.config.output_policy = policy;
        self
    }

    #[must_use]
    pub fn scale_factor(mut self, sf: impl TryInto<ScaleFactor, Error = DomainError>) -> Self {
        self.config.scale_factor = sf.try_into().unwrap_or_default();
        self
    }

    pub fn apply(self) -> Result<()> {
        self.handle.apply_config(self.config)
    }
}

pub struct EventDispatchContext<'a> {
    app_state: &'a mut AppState,
}

impl<'a> FromAppState<'a> for EventDispatchContext<'a> {
    fn from_app_state(app_state: &'a mut AppState) -> Self {
        Self { app_state }
    }
}

fn extract_dimensions_from_callback(args: &[Value]) -> PopupDimensions {
    let defaults = PopupDimensions::default();
    PopupDimensions::new(
        args.first()
            .and_then(|v| v.clone().try_into().ok())
            .unwrap_or(defaults.width),
        args.get(1)
            .and_then(|v| v.clone().try_into().ok())
            .unwrap_or(defaults.height),
    )
}

impl EventDispatchContext<'_> {
    pub(crate) fn surfaces_by_name(&self, name: &str) -> impl Iterator<Item = &SurfaceState> {
        self.app_state.surfaces_by_name(name)
    }

    pub(crate) fn surfaces_by_name_mut(
        &mut self,
        name: &str,
    ) -> impl Iterator<Item = &mut SurfaceState> {
        self.app_state.surfaces_by_name_mut(name)
    }

    pub fn with_surface<F, R>(&self, name: &str, f: F) -> Result<R>
    where
        F: FnOnce(&ComponentInstance) -> R,
    {
        let component = self.get_surface_component(name).ok_or_else(|| {
            Error::Domain(DomainError::Configuration {
                message: format!("Surface '{}' not found", name),
            })
        })?;
        Ok(f(component))
    }

    pub fn with_output<F, R>(&self, handle: OutputHandle, f: F) -> Result<R>
    where
        F: FnOnce(&ComponentInstance) -> R,
    {
        let component = self.get_output_component(handle).ok_or_else(|| {
            Error::Domain(DomainError::Configuration {
                message: format!("Output with handle {:?} not found", handle),
            })
        })?;
        Ok(f(component))
    }

    fn get_surface_component(&self, name: &str) -> Option<&ComponentInstance> {
        self.app_state
            .surfaces_by_name(name)
            .next()
            .map(SurfaceState::component_instance)
    }

    #[must_use]
    pub fn component_instance(&self) -> Option<&ComponentInstance> {
        self.app_state
            .primary_output()
            .map(SurfaceState::component_instance)
    }

    pub fn all_component_instances(&self) -> impl Iterator<Item = &ComponentInstance> {
        self.app_state
            .all_outputs()
            .map(SurfaceState::component_instance)
    }

    pub const fn output_registry(&self) -> &OutputRegistry {
        self.app_state.output_registry()
    }

    #[must_use]
    pub fn primary_output_handle(&self) -> Option<OutputHandle> {
        self.app_state.primary_output_handle()
    }

    #[must_use]
    pub fn active_output_handle(&self) -> Option<OutputHandle> {
        self.app_state.active_output_handle()
    }

    pub fn outputs(&self) -> impl Iterator<Item = (OutputHandle, &ComponentInstance)> {
        self.app_state
            .outputs_with_handles()
            .map(|(handle, surface)| (handle, surface.component_instance()))
    }

    pub fn get_output_component(&self, handle: OutputHandle) -> Option<&ComponentInstance> {
        self.app_state
            .get_output_by_handle(handle)
            .map(SurfaceState::component_instance)
    }

    pub fn get_output_info(&self, handle: OutputHandle) -> Option<&OutputInfo> {
        self.app_state.get_output_info(handle)
    }

    pub fn all_output_info(&self) -> impl Iterator<Item = &OutputInfo> {
        self.app_state.all_output_info()
    }

    pub fn outputs_with_info(&self) -> impl Iterator<Item = (&OutputInfo, &ComponentInstance)> {
        self.app_state
            .outputs_with_info()
            .map(|(info, surface)| (info, surface.component_instance()))
    }

    fn active_or_primary_output(&self) -> Option<&SurfaceState> {
        self.app_state
            .active_output()
            .or_else(|| self.app_state.primary_output())
    }

    pub fn render_frame_if_dirty(&mut self) -> Result<()> {
        for surface in self.app_state.all_outputs() {
            surface.render_frame_if_dirty()?;
        }
        Ok(())
    }

    #[must_use]
    pub fn compilation_result(&self) -> Option<Rc<CompilationResult>> {
        self.app_state
            .primary_output()
            .and_then(SurfaceState::compilation_result)
    }

    pub fn show_popup(
        &mut self,
        req: &PopupRequest,
        resize_control: Option<ShellControl>,
    ) -> Result<PopupHandle> {
        log::info!("show_popup called for component '{}'", req.component);

        let compilation_result = self.compilation_result().ok_or_else(|| {
            log::error!("No compilation result available");
            Error::Domain(DomainError::Configuration {
                message: "No compilation result available for popup creation".to_string(),
            })
        })?;

        log::debug!(
            "Got compilation result, looking for component '{}'",
            req.component
        );

        let definition = compilation_result
            .component(&req.component)
            .ok_or_else(|| {
                log::error!(
                    "Component '{}' not found in compilation result",
                    req.component
                );
                Error::Domain(DomainError::Configuration {
                    message: format!(
                        "{} component not found in compilation result",
                        req.component
                    ),
                })
            })?;

        log::debug!("Found component definition for '{}'", req.component);

        self.close_current_popup()?;

        let is_using_active = self.app_state.active_output().is_some();
        let active_surface = self.active_or_primary_output().ok_or_else(|| {
            log::error!("No active or primary output available");
            Error::Domain(DomainError::Configuration {
                message: "No active or primary output available".to_string(),
            })
        })?;

        log::info!(
            "Creating popup on {} output",
            if is_using_active { "active" } else { "primary" }
        );

        let popup_manager = active_surface.popup_manager().ok_or_else(|| {
            Error::Domain(DomainError::Configuration {
                message: "No popup manager available".to_string(),
            })
        })?;

        let initial_dimensions = match req.size {
            PopupSize::Fixed { w, h } => {
                log::debug!("Using fixed popup size: {}x{}", w, h);
                (w, h)
            }
            PopupSize::Content => {
                log::debug!("Using content-based sizing - will measure after instance creation");
                (2.0, 2.0)
            }
        };

        log::debug!(
            "Creating popup for '{}' with dimensions {}x{} at position ({}, {}), mode: {:?}",
            req.component,
            initial_dimensions.0,
            initial_dimensions.1,
            req.placement.position().0,
            req.placement.position().1,
            req.mode
        );

        let popup_handle =
            popup_manager.request_popup(req.clone(), initial_dimensions.0, initial_dimensions.1);

        let (instance, popup_key_cell) =
            Self::create_popup_instance(&definition, &popup_manager, resize_control, req)?;

        popup_key_cell.set(popup_handle.key());

        if let Some(popup_surface) = popup_manager.get_popup_window(popup_handle.key()) {
            popup_surface.set_component_instance(instance);
        } else {
            return Err(Error::Domain(DomainError::Configuration {
                message: "Popup window not found after creation".to_string(),
            }));
        }

        Ok(popup_handle)
    }

    pub fn close_popup(&mut self, handle: PopupHandle) -> Result<()> {
        if let Some(active_surface) = self.active_or_primary_output() {
            if let Some(popup_manager) = active_surface.popup_manager() {
                popup_manager.close(handle)?;
            }
        }
        Ok(())
    }

    pub fn close_current_popup(&mut self) -> Result<()> {
        if let Some(active_surface) = self.active_or_primary_output() {
            if let Some(popup_manager) = active_surface.popup_manager() {
                popup_manager.close_current_popup();
            }
        }
        Ok(())
    }

    pub fn resize_popup(&mut self, handle: PopupHandle, width: f32, height: f32) -> Result<()> {
        let active_surface = self.active_or_primary_output().ok_or_else(|| {
            Error::Domain(DomainError::Configuration {
                message: "No active or primary output available".to_string(),
            })
        })?;

        let popup_manager = active_surface.popup_manager().ok_or_else(|| {
            Error::Domain(DomainError::Configuration {
                message: "No popup manager available".to_string(),
            })
        })?;

        let Some((request, _serial)) = popup_manager.get_popup_info(handle.key()) else {
            log::debug!(
                "Ignoring resize request for non-existent popup with handle {:?}",
                handle
            );
            return Ok(());
        };

        let current_size = request.size.dimensions();
        let size_changed =
            current_size.is_none_or(|(w, h)| (w - width).abs() > 0.01 || (h - height).abs() > 0.01);

        if size_changed {
            if let Some(popup_surface) = popup_manager.get_popup_window(handle.key()) {
                popup_surface.request_resize(width, height);

                #[allow(clippy::cast_possible_truncation)]
                #[allow(clippy::cast_possible_wrap)]
                let logical_width = width as i32;
                #[allow(clippy::cast_possible_truncation)]
                #[allow(clippy::cast_possible_wrap)]
                let logical_height = height as i32;

                popup_manager.update_popup_viewport(handle.key(), logical_width, logical_height);
                popup_manager.commit_popup_surface(handle.key());
                log::debug!(
                    "Updated popup viewport to logical size: {}x{} (from resize to {}x{})",
                    logical_width,
                    logical_height,
                    width,
                    height
                );
            }
        }

        Ok(())
    }

    fn create_popup_instance(
        definition: &ComponentDefinition,
        popup_manager: &Rc<PopupManager>,
        resize_control: Option<ShellControl>,
        req: &PopupRequest,
    ) -> Result<(ComponentInstance, Rc<Cell<usize>>)> {
        let instance = definition.create().map_err(|e| {
            Error::Domain(DomainError::Configuration {
                message: format!("Failed to create popup instance: {}", e),
            })
        })?;

        let popup_key_cell = Rc::new(Cell::new(0));

        Self::register_popup_callbacks(
            &instance,
            popup_manager,
            resize_control,
            &popup_key_cell,
            req,
        )?;

        instance.show().map_err(|e| {
            Error::Domain(DomainError::Configuration {
                message: format!("Failed to show popup instance: {}", e),
            })
        })?;

        Ok((instance, popup_key_cell))
    }

    fn register_popup_callbacks(
        instance: &ComponentInstance,
        popup_manager: &Rc<PopupManager>,
        resize_control: Option<ShellControl>,
        popup_key_cell: &Rc<Cell<usize>>,
        req: &PopupRequest,
    ) -> Result<()> {
        if let Some(close_callback_name) = &req.close_callback {
            Self::register_close_callback(instance, popup_manager, close_callback_name)?;
        }

        if let Some(resize_callback_name) = &req.resize_callback {
            Self::register_resize_callback(
                instance,
                popup_manager,
                resize_control,
                popup_key_cell,
                resize_callback_name,
            )?;
        }

        Ok(())
    }

    fn register_close_callback(
        instance: &ComponentInstance,
        popup_manager: &Rc<PopupManager>,
        callback_name: &str,
    ) -> Result<()> {
        let popup_manager_weak = Rc::downgrade(popup_manager);
        instance
            .set_callback(callback_name, move |_| {
                if let Some(popup_manager) = popup_manager_weak.upgrade() {
                    popup_manager.close_current_popup();
                }
                Value::Void
            })
            .map_err(|e| {
                Error::Domain(DomainError::Configuration {
                    message: format!("Failed to set '{}' callback: {}", callback_name, e),
                })
            })
    }

    fn register_resize_callback(
        instance: &ComponentInstance,
        popup_manager: &Rc<PopupManager>,
        resize_control: Option<ShellControl>,
        popup_key_cell: &Rc<Cell<usize>>,
        callback_name: &str,
    ) -> Result<()> {
        if let Some(control) = resize_control {
            Self::register_resize_with_control(instance, popup_key_cell, &control, callback_name)
        } else {
            Self::register_resize_direct(instance, popup_manager, popup_key_cell, callback_name)
        }
    }

    fn register_resize_with_control(
        instance: &ComponentInstance,
        popup_key_cell: &Rc<Cell<usize>>,
        control: &ShellControl,
        callback_name: &str,
    ) -> Result<()> {
        let key_cell = Rc::clone(popup_key_cell);
        let control = control.clone();
        instance
            .set_callback(callback_name, move |args| {
                let dimensions = extract_dimensions_from_callback(args);
                let popup_key = key_cell.get();

                log::info!(
                    "Resize callback invoked: {}x{} for key {}",
                    dimensions.width,
                    dimensions.height,
                    popup_key
                );

                if control
                    .resize_popup(
                        PopupHandle::from_raw(popup_key),
                        dimensions.width,
                        dimensions.height,
                    )
                    .is_err()
                {
                    log::error!("Failed to resize popup through control");
                }
                Value::Void
            })
            .map_err(|e| {
                Error::Domain(DomainError::Configuration {
                    message: format!("Failed to set '{}' callback: {}", callback_name, e),
                })
            })
    }

    fn register_resize_direct(
        instance: &ComponentInstance,
        popup_manager: &Rc<PopupManager>,
        popup_key_cell: &Rc<Cell<usize>>,
        callback_name: &str,
    ) -> Result<()> {
        let popup_manager_weak = Rc::downgrade(popup_manager);
        let key_cell = Rc::clone(popup_key_cell);
        instance
            .set_callback(callback_name, move |args| {
                let dimensions = extract_dimensions_from_callback(args);
                let popup_key = key_cell.get();

                log::info!(
                    "Resize callback invoked: {}x{} for key {}",
                    dimensions.width,
                    dimensions.height,
                    popup_key
                );

                if let Some(popup_manager) = popup_manager_weak.upgrade() {
                    if let Some(popup_surface) = popup_manager.get_popup_window(popup_key) {
                        popup_surface.request_resize(dimensions.width, dimensions.height);

                        #[allow(clippy::cast_possible_truncation)]
                        #[allow(clippy::cast_possible_wrap)]
                        let logical_width = dimensions.width as i32;
                        #[allow(clippy::cast_possible_truncation)]
                        #[allow(clippy::cast_possible_wrap)]
                        let logical_height = dimensions.height as i32;

                        popup_manager.update_popup_viewport(
                            popup_key,
                            logical_width,
                            logical_height,
                        );
                        log::debug!(
                            "Updated popup viewport to logical size: {}x{} (from direct resize to {}x{})",
                            logical_width,
                            logical_height,
                            dimensions.width,
                            dimensions.height
                        );
                    }
                }
                Value::Void
            })
            .map_err(|e| {
                Error::Domain(DomainError::Configuration {
                    message: format!("Failed to set '{}' callback: {}", callback_name, e),
                })
            })
    }

    pub fn configure_surface<F>(&mut self, name: &str, f: F) -> Result<()>
    where
        F: FnOnce(&ComponentInstance, LayerSurfaceHandle<'_>),
    {
        let surface = self
            .app_state
            .surfaces_by_name(name)
            .next()
            .ok_or_else(|| {
                Error::Domain(DomainError::Configuration {
                    message: format!("Surface '{}' not found", name),
                })
            })?;

        let handle = LayerSurfaceHandle::from_window_state(surface);
        let component = surface.component_instance();
        f(component, handle);
        Ok(())
    }

    pub fn configure_all_surfaces<F>(&mut self, mut f: F)
    where
        F: FnMut(&ComponentInstance, LayerSurfaceHandle<'_>),
    {
        for surface in self.app_state.all_outputs() {
            let handle = LayerSurfaceHandle::from_window_state(surface);
            let component = surface.component_instance();
            f(component, handle);
        }
    }
}
