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
use layer_shika_domain::value_objects::handle::SurfaceHandle;
use layer_shika_domain::value_objects::output_handle::OutputHandle;
use layer_shika_domain::value_objects::output_info::OutputInfo;
use layer_shika_domain::value_objects::popup_positioning_mode::PopupPositioningMode;
use layer_shika_domain::value_objects::popup_request::{
    PopupHandle, PopupPlacement, PopupRequest, PopupSize,
};
use layer_shika_domain::value_objects::surface_instance_id::SurfaceInstanceId;
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

#[derive(Debug, Clone)]
pub enum SurfaceTarget {
    ByInstance(SurfaceInstanceId),
    ByHandle(SurfaceHandle),
    ByName(String),
    ByNameAndOutput { name: String, output: OutputHandle },
}

pub enum SurfaceCommand {
    Resize {
        target: SurfaceTarget,
        width: u32,
        height: u32,
    },
    SetAnchor {
        target: SurfaceTarget,
        anchor: AnchorEdges,
    },
    SetExclusiveZone {
        target: SurfaceTarget,
        zone: i32,
    },
    SetMargins {
        target: SurfaceTarget,
        margins: Margins,
    },
    SetLayer {
        target: SurfaceTarget,
        layer: Layer,
    },
    SetOutputPolicy {
        target: SurfaceTarget,
        policy: OutputPolicy,
    },
    SetScaleFactor {
        target: SurfaceTarget,
        factor: ScaleFactor,
    },
    SetKeyboardInteractivity {
        target: SurfaceTarget,
        mode: KeyboardInteractivity,
    },
    ApplyConfig {
        target: SurfaceTarget,
        config: SurfaceConfig,
    },
}

pub enum ShellCommand {
    Popup(PopupCommand),
    Surface(SurfaceCommand),
    Render,
}

/// Context provided to callback handlers with surface and control information
///
/// Provides surface identity, output info, and control handle for runtime operations.
pub struct CallbackContext {
    instance_id: SurfaceInstanceId,
    surface_name: String,
    control: ShellControl,
}

impl CallbackContext {
    pub fn new(
        instance_id: SurfaceInstanceId,
        surface_name: String,
        control: ShellControl,
    ) -> Self {
        Self {
            instance_id,
            surface_name,
            control,
        }
    }

    /// Returns the surface instance identifier
    pub const fn instance_id(&self) -> &SurfaceInstanceId {
        &self.instance_id
    }

    /// Returns the surface handle
    pub const fn surface_handle(&self) -> SurfaceHandle {
        self.instance_id.surface()
    }

    /// Returns the output handle
    pub const fn output_handle(&self) -> OutputHandle {
        self.instance_id.output()
    }

    /// Returns the surface name
    pub fn surface_name(&self) -> &str {
        &self.surface_name
    }

    /// Returns a reference to the shell control handle
    pub const fn control(&self) -> &ShellControl {
        &self.control
    }

    /// Returns a control handle for this specific surface instance
    pub fn this_instance(&self) -> SurfaceControlHandle {
        self.control.surface_instance(&self.instance_id)
    }

    /// Returns a control handle for all instances of this surface
    pub fn all_surface_instances(&self) -> SurfaceControlHandle {
        self.control.surface_by_handle(self.surface_handle())
    }

    /// Returns a control handle for all surfaces with this name
    pub fn all_named(&self) -> SurfaceControlHandle {
        self.control.surface_by_name(&self.surface_name)
    }

    /// Returns a control handle for all surfaces with this name on the current output
    pub fn all_named_on_this_output(&self) -> SurfaceControlHandle {
        self.control
            .surface_by_name_and_output(&self.surface_name, self.output_handle())
    }

    /// Shows a popup from a popup request
    ///
    /// Convenience method that forwards to the underlying `ShellControl`.
    /// See [`ShellControl::show_popup`] for full documentation.
    pub fn show_popup(&self, request: &PopupRequest) -> Result<()> {
        self.control.show_popup(request)
    }

    /// Shows a popup at the current cursor position
    ///
    /// Convenience method that forwards to the underlying `ShellControl`.
    /// See [`ShellControl::show_popup_at_cursor`] for full documentation.
    pub fn show_popup_at_cursor(&self, component: impl Into<String>) -> Result<()> {
        self.control.show_popup_at_cursor(component)
    }

    /// Shows a popup centered on screen
    ///
    /// Convenience method that forwards to the underlying `ShellControl`.
    /// See [`ShellControl::show_popup_centered`] for full documentation.
    pub fn show_popup_centered(&self, component: impl Into<String>) -> Result<()> {
        self.control.show_popup_centered(component)
    }

    /// Shows a popup at the specified absolute position
    ///
    /// Convenience method that forwards to the underlying `ShellControl`.
    /// See [`ShellControl::show_popup_at_position`] for full documentation.
    pub fn show_popup_at_position(
        &self,
        component: impl Into<String>,
        x: f32,
        y: f32,
    ) -> Result<()> {
        self.control.show_popup_at_position(component, x, y)
    }

    /// Closes a specific popup by its handle
    ///
    /// Convenience method that forwards to the underlying `ShellControl`.
    /// See [`ShellControl::close_popup`] for full documentation.
    pub fn close_popup(&self, handle: PopupHandle) -> Result<()> {
        self.control.close_popup(handle)
    }

    /// Resizes a popup to the specified dimensions
    ///
    /// Convenience method that forwards to the underlying `ShellControl`.
    /// See [`ShellControl::resize_popup`] for full documentation.
    pub fn resize_popup(&self, handle: PopupHandle, width: f32, height: f32) -> Result<()> {
        self.control.resize_popup(handle, width, height)
    }
}

/// Handle for runtime control of shell operations
///
/// Cloneable and can be sent across threads for triggering shell operations.
#[derive(Clone)]
pub struct ShellControl {
    sender: channel::Sender<ShellCommand>,
}

impl ShellControl {
    pub fn new(sender: channel::Sender<ShellCommand>) -> Self {
        Self { sender }
    }

    /// Shows a popup from a popup request
    ///
    /// This is the primary API for showing popups from Slint callbacks. Popups are
    /// transient windows that appear above the main surface, commonly used for menus,
    /// tooltips, dropdowns, and other temporary UI elements.
    ///
    /// # Content-Based Sizing
    ///
    /// When using `PopupSize::Content`, you must configure a resize callback via
    /// `resize_on()` to enable automatic resizing. The popup component should use a
    /// `Timer` with `interval: 1ms` to invoke the resize callback after initialization,
    /// ensuring the component is initialized before callback invocation. This allows the
    /// popup to reposition itself to fit the content. See the `popup-demo` example.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// shell.on("Main", "open_menu", |control| {
    ///     let request = PopupRequest::builder("MenuPopup")
    ///         .placement(PopupPlacement::at_cursor())
    ///         .grab(true)
    ///         .close_on("menu_closed")
    ///         .build();
    ///
    ///     control.show_popup(&request)?;
    ///     Value::Void
    /// });
    /// ```
    ///
    /// # See Also
    ///
    /// - [`show_popup_at_cursor`](Self::show_popup_at_cursor) - Convenience method for cursor-positioned popups
    /// - [`show_popup_centered`](Self::show_popup_centered) - Convenience method for centered popups
    /// - [`show_popup_at_position`](Self::show_popup_at_position) - Convenience method for absolute positioning
    /// - [`PopupRequest`] - Full popup configuration options
    /// - [`PopupBuilder`] - Fluent API for building popup requests
    pub fn show_popup(&self, request: &PopupRequest) -> Result<()> {
        self.sender
            .send(ShellCommand::Popup(PopupCommand::Show(request.clone())))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send popup show command: channel closed".to_string(),
                })
            })
    }

    /// Shows a popup at the current cursor position
    ///
    /// Convenience method for showing a popup at the cursor with default settings.
    /// For more control over popup positioning, sizing, and behavior, use
    /// [`show_popup`](Self::show_popup) with a [`PopupRequest`].
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// shell.on("Main", "context_menu", |control| {
    ///     control.show_popup_at_cursor("ContextMenu")?;
    ///     Value::Void
    /// });
    /// ```
    pub fn show_popup_at_cursor(&self, component: impl Into<String>) -> Result<()> {
        let request = PopupRequest::builder(component.into())
            .placement(PopupPlacement::AtCursor)
            .build();
        self.show_popup(&request)
    }

    /// Shows a popup centered on screen
    ///
    /// Convenience method for showing a centered popup. Useful for dialogs
    /// and modal content that should appear in the middle of the screen.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// shell.on("Main", "show_dialog", |control| {
    ///     control.show_popup_centered("ConfirmDialog")?;
    ///     Value::Void
    /// });
    /// ```
    pub fn show_popup_centered(&self, component: impl Into<String>) -> Result<()> {
        let request = PopupRequest::builder(component.into())
            .placement(PopupPlacement::AtCursor)
            .mode(PopupPositioningMode::Center)
            .build();
        self.show_popup(&request)
    }

    /// Shows a popup at the specified absolute position
    ///
    /// Convenience method for showing a popup at an exact screen coordinate.
    /// The position is in logical pixels relative to the surface origin.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// shell.on("Main", "show_tooltip", |control| {
    ///     control.show_popup_at_position("Tooltip", 100.0, 50.0)?;
    ///     Value::Void
    /// });
    /// ```
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

    /// Closes a specific popup by its handle
    ///
    /// Use this when you need to close a specific popup that you opened previously.
    /// The handle is returned by [`show_popup`](Self::show_popup) and related methods.
    ///
    /// For closing popups from within the popup itself, consider using the
    /// `close_on` callback configuration in [`PopupRequest`] instead.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Store handle when showing popup
    /// let handle = context.show_popup(&request)?;
    ///
    /// // Later, close it
    /// control.close_popup(handle)?;
    /// ```
    pub fn close_popup(&self, handle: PopupHandle) -> Result<()> {
        self.sender
            .send(ShellCommand::Popup(PopupCommand::Close(handle)))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send popup close command: channel closed".to_string(),
                })
            })
    }

    /// Resizes a popup to the specified dimensions
    ///
    /// Dynamically changes the size of an active popup. This is typically used
    /// in response to content changes or user interaction.
    ///
    /// For automatic content-based sizing, use `PopupSize::Content` with the
    /// `resize_on` callback configuration in [`PopupRequest`] instead.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// shell.on("Main", "expand_menu", |control| {
    ///     // Assuming we have the popup handle stored somewhere
    ///     control.resize_popup(menu_handle, 400.0, 600.0)?;
    ///     Value::Void
    /// });
    /// ```
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

    /// Requests a redraw of all surfaces
    pub fn request_redraw(&self) -> Result<()> {
        self.sender.send(ShellCommand::Render).map_err(|_| {
            Error::Domain(DomainError::Configuration {
                message: "Failed to send redraw command: channel closed".to_string(),
            })
        })
    }

    /// Returns a control handle for a specific surface instance
    pub fn surface_instance(&self, id: &SurfaceInstanceId) -> SurfaceControlHandle {
        SurfaceControlHandle {
            target: SurfaceTarget::ByInstance(*id),
            sender: self.sender.clone(),
        }
    }

    /// Returns a control handle for all instances of a surface by handle
    pub fn surface_by_handle(&self, handle: SurfaceHandle) -> SurfaceControlHandle {
        SurfaceControlHandle {
            target: SurfaceTarget::ByHandle(handle),
            sender: self.sender.clone(),
        }
    }

    /// Returns a control handle for all surfaces with the given name
    pub fn surface_by_name(&self, name: impl Into<String>) -> SurfaceControlHandle {
        SurfaceControlHandle {
            target: SurfaceTarget::ByName(name.into()),
            sender: self.sender.clone(),
        }
    }

    /// Returns a control handle for surfaces with the given name on a specific output
    pub fn surface_by_name_and_output(
        &self,
        name: impl Into<String>,
        output: OutputHandle,
    ) -> SurfaceControlHandle {
        SurfaceControlHandle {
            target: SurfaceTarget::ByNameAndOutput {
                name: name.into(),
                output,
            },
            sender: self.sender.clone(),
        }
    }

    /// Alias for `surface_by_name`
    pub fn surface(&self, name: impl Into<String>) -> SurfaceControlHandle {
        self.surface_by_name(name)
    }
}

/// Handle for runtime control of a specific surface
///
/// Operations apply to all matching instances. Changes are queued and applied asynchronously.
/// Obtained via `ShellControl::surface()`.
pub struct SurfaceControlHandle {
    target: SurfaceTarget,
    sender: channel::Sender<ShellCommand>,
}

impl SurfaceControlHandle {
    /// Resizes the surface to the specified dimensions
    pub fn resize(&self, width: u32, height: u32) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(SurfaceCommand::Resize {
                target: self.target.clone(),
                width,
                height,
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send surface resize command: channel closed".to_string(),
                })
            })
    }

    /// Sets the surface width
    pub fn set_width(&self, width: u32) -> Result<()> {
        self.resize(width, 0)
    }

    /// Sets the surface height
    pub fn set_height(&self, height: u32) -> Result<()> {
        self.resize(0, height)
    }

    /// Sets the anchor edges for the surface
    pub fn set_anchor(&self, anchor: AnchorEdges) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(SurfaceCommand::SetAnchor {
                target: self.target.clone(),
                anchor,
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send surface set_anchor command: channel closed"
                        .to_string(),
                })
            })
    }

    /// Sets the exclusive zone for the surface
    pub fn set_exclusive_zone(&self, zone: i32) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(SurfaceCommand::SetExclusiveZone {
                target: self.target.clone(),
                zone,
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send surface set_exclusive_zone command: channel closed"
                        .to_string(),
                })
            })
    }

    /// Sets the margins for the surface
    pub fn set_margins(&self, margins: impl Into<Margins>) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(SurfaceCommand::SetMargins {
                target: self.target.clone(),
                margins: margins.into(),
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send surface set_margins command: channel closed"
                        .to_string(),
                })
            })
    }

    /// Sets the layer for the surface
    pub fn set_layer(&self, layer: Layer) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(SurfaceCommand::SetLayer {
                target: self.target.clone(),
                layer,
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send surface set_layer command: channel closed".to_string(),
                })
            })
    }

    /// Sets the output policy for the surface
    pub fn set_output_policy(&self, policy: OutputPolicy) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(SurfaceCommand::SetOutputPolicy {
                target: self.target.clone(),
                policy,
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send surface set_output_policy command: channel closed"
                        .to_string(),
                })
            })
    }

    /// Sets the scale factor for the surface
    pub fn set_scale_factor(&self, factor: ScaleFactor) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(SurfaceCommand::SetScaleFactor {
                target: self.target.clone(),
                factor,
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send surface set_scale_factor command: channel closed"
                        .to_string(),
                })
            })
    }

    /// Sets the keyboard interactivity mode for the surface
    pub fn set_keyboard_interactivity(&self, mode: KeyboardInteractivity) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(
                SurfaceCommand::SetKeyboardInteractivity {
                    target: self.target.clone(),
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

    /// Applies a complete surface configuration
    pub fn apply_config(&self, config: SurfaceConfig) -> Result<()> {
        self.sender
            .send(ShellCommand::Surface(SurfaceCommand::ApplyConfig {
                target: self.target.clone(),
                config,
            }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send surface apply_config command: channel closed"
                        .to_string(),
                })
            })
    }

    /// Returns a builder for configuring multiple properties at once
    pub fn configure(self) -> RuntimeSurfaceConfigBuilder {
        RuntimeSurfaceConfigBuilder {
            handle: self,
            config: SurfaceConfig::new(),
        }
    }
}

/// Builder for applying multiple configuration changes to a surface at once
///
/// Created via `SurfaceControlHandle::configure()`. Chain configuration methods
/// and call `.apply()` to commit all changes atomically.
/// Builder for applying multiple configuration changes to a surface at once
///
/// All changes are committed together in one compositor round-trip for efficiency.
pub struct RuntimeSurfaceConfigBuilder {
    handle: SurfaceControlHandle,
    config: SurfaceConfig,
}

impl RuntimeSurfaceConfigBuilder {
    /// Sets the surface size
    #[must_use]
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.config.dimensions = SurfaceDimension::from_raw(width, height);
        self
    }

    /// Sets the surface width
    #[must_use]
    pub fn width(mut self, width: u32) -> Self {
        self.config.dimensions = SurfaceDimension::from_raw(width, self.config.dimensions.height());
        self
    }

    /// Sets the surface height
    #[must_use]
    pub fn height(mut self, height: u32) -> Self {
        self.config.dimensions = SurfaceDimension::from_raw(self.config.dimensions.width(), height);
        self
    }

    /// Sets the layer
    #[must_use]
    pub const fn layer(mut self, layer: Layer) -> Self {
        self.config.layer = layer;
        self
    }

    /// Sets the margins
    #[must_use]
    pub fn margins(mut self, margins: impl Into<Margins>) -> Self {
        self.config.margin = margins.into();
        self
    }

    /// Sets the anchor edges
    #[must_use]
    pub const fn anchor(mut self, anchor: AnchorEdges) -> Self {
        self.config.anchor = anchor;
        self
    }

    /// Sets the exclusive zone
    #[must_use]
    pub const fn exclusive_zone(mut self, zone: i32) -> Self {
        self.config.exclusive_zone = zone;
        self
    }

    /// Sets the namespace
    #[must_use]
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.config.namespace = namespace.into();
        self
    }

    /// Sets the keyboard interactivity mode
    #[must_use]
    pub const fn keyboard_interactivity(mut self, mode: KeyboardInteractivity) -> Self {
        self.config.keyboard_interactivity = mode;
        self
    }

    /// Sets the output policy
    #[must_use]
    pub fn output_policy(mut self, policy: OutputPolicy) -> Self {
        self.config.output_policy = policy;
        self
    }

    /// Sets the scale factor
    #[must_use]
    pub fn scale_factor(mut self, sf: impl TryInto<ScaleFactor, Error = DomainError>) -> Self {
        self.config.scale_factor = sf.try_into().unwrap_or_default();
        self
    }

    /// Applies the configured changes to the surface
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
    pub(crate) fn surface_by_instance_mut(
        &mut self,
        surface_handle: SurfaceHandle,
        output_handle: OutputHandle,
    ) -> Option<&mut SurfaceState> {
        self.app_state
            .get_surface_by_instance_mut(surface_handle, output_handle)
    }

    pub(crate) fn surfaces_by_handle_mut(
        &mut self,
        handle: SurfaceHandle,
    ) -> Vec<&mut SurfaceState> {
        self.app_state.surfaces_by_handle_mut(handle)
    }

    pub(crate) fn surfaces_by_name_mut(&mut self, name: &str) -> Vec<&mut SurfaceState> {
        self.app_state.surfaces_by_name_mut(name)
    }

    pub(crate) fn surfaces_by_name_and_output_mut(
        &mut self,
        name: &str,
        output: OutputHandle,
    ) -> Vec<&mut SurfaceState> {
        self.app_state.surfaces_by_name_and_output_mut(name, output)
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
            .first()
            .map(|s| s.component_instance())
    }

    /// Returns the primary component instance
    #[must_use]
    pub fn component_instance(&self) -> Option<&ComponentInstance> {
        self.app_state
            .primary_output()
            .map(SurfaceState::component_instance)
    }

    /// Returns all component instances across all outputs
    pub fn all_component_instances(&self) -> impl Iterator<Item = &ComponentInstance> {
        self.app_state
            .all_outputs()
            .map(SurfaceState::component_instance)
    }

    /// Returns the output registry
    pub const fn output_registry(&self) -> &OutputRegistry {
        self.app_state.output_registry()
    }

    /// Returns the primary output handle
    #[must_use]
    pub fn primary_output_handle(&self) -> Option<OutputHandle> {
        self.app_state.primary_output_handle()
    }

    /// Returns the active output handle
    #[must_use]
    pub fn active_output_handle(&self) -> Option<OutputHandle> {
        self.app_state.active_output_handle()
    }

    /// Returns all outputs with their handles and components
    pub fn outputs(&self) -> impl Iterator<Item = (OutputHandle, &ComponentInstance)> {
        self.app_state
            .outputs_with_handles()
            .map(|(handle, surface)| (handle, surface.component_instance()))
    }

    /// Returns the component for a specific output
    pub fn get_output_component(&self, handle: OutputHandle) -> Option<&ComponentInstance> {
        self.app_state
            .get_output_by_handle(handle)
            .map(SurfaceState::component_instance)
    }

    /// Returns information about a specific output
    pub fn get_output_info(&self, handle: OutputHandle) -> Option<&OutputInfo> {
        self.app_state.get_output_info(handle)
    }

    /// Returns information about all outputs
    pub fn all_output_info(&self) -> impl Iterator<Item = &OutputInfo> {
        self.app_state.all_output_info()
    }

    /// Returns all outputs with their info and components
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

    /// Renders a new frame for all dirty surfaces
    pub fn render_frame_if_dirty(&mut self) -> Result<()> {
        for surface in self.app_state.all_outputs() {
            surface.render_frame_if_dirty()?;
        }
        Ok(())
    }

    /// Returns the compilation result if available
    #[must_use]
    pub fn compilation_result(&self) -> Option<Rc<CompilationResult>> {
        self.app_state
            .primary_output()
            .and_then(SurfaceState::compilation_result)
    }

    /// Shows a popup from a popup request
    ///
    /// Resize callbacks (if configured via `resize_on()`) will operate directly
    /// on the popup manager for immediate updates.
    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    pub fn show_popup(&mut self, req: &PopupRequest) -> Result<PopupHandle> {
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

        // For content-based sizing, we need to query the component's preferred size first
        let initial_dimensions = match req.size {
            PopupSize::Fixed { w, h } => {
                log::debug!("Using fixed popup size: {}x{}", w, h);
                (w, h)
            }
            PopupSize::Content => {
                log::debug!("Using content-based sizing - starting at 2×2");
                // Start with minimal size. Consumer app should register a callback to
                // call resize_popup() with the desired dimensions.
                (2.0, 2.0)
            }
        };

        let resolved_placement = match req.placement {
            PopupPlacement::AtCursor => {
                let cursor_pos = active_surface.current_pointer_position();
                log::debug!(
                    "Resolving AtCursor placement to actual cursor position: ({}, {})",
                    cursor_pos.x,
                    cursor_pos.y
                );
                PopupPlacement::AtPosition {
                    x: cursor_pos.x,
                    y: cursor_pos.y,
                }
            }
            other => other,
        };

        let (ref_x, ref_y) = resolved_placement.position();

        log::debug!(
            "Creating popup for '{}' with dimensions {}x{} at position ({}, {}), mode: {:?}",
            req.component,
            initial_dimensions.0,
            initial_dimensions.1,
            ref_x,
            ref_y,
            req.mode
        );

        // Create a new request with resolved placement
        let resolved_request = PopupRequest {
            component: req.component.clone(),
            placement: resolved_placement,
            size: req.size,
            mode: req.mode,
            grab: req.grab,
            close_callback: req.close_callback.clone(),
            resize_callback: req.resize_callback.clone(),
        };

        let popup_handle = popup_manager.request_popup(
            resolved_request,
            initial_dimensions.0,
            initial_dimensions.1,
        );

        let (instance, popup_key_cell) =
            Self::create_popup_instance(&definition, &popup_manager, req)?;

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

    /// Closes a popup by its handle
    pub fn close_popup(&mut self, handle: PopupHandle) -> Result<()> {
        if let Some(active_surface) = self.active_or_primary_output() {
            if let Some(popup_manager) = active_surface.popup_manager() {
                popup_manager.close(handle)?;
            }
        }
        Ok(())
    }

    /// Closes the currently active popup
    pub fn close_current_popup(&mut self) -> Result<()> {
        if let Some(active_surface) = self.active_or_primary_output() {
            if let Some(popup_manager) = active_surface.popup_manager() {
                popup_manager.close_current_popup();
            }
        }
        Ok(())
    }

    /// Resizes a popup to the specified dimensions
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
        req: &PopupRequest,
    ) -> Result<(ComponentInstance, Rc<Cell<usize>>)> {
        let instance = definition.create().map_err(|e| {
            Error::Domain(DomainError::Configuration {
                message: format!("Failed to create popup instance: {}", e),
            })
        })?;

        let popup_key_cell = Rc::new(Cell::new(0));

        Self::register_popup_callbacks(&instance, popup_manager, &popup_key_cell, req)?;

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
        popup_key_cell: &Rc<Cell<usize>>,
        req: &PopupRequest,
    ) -> Result<()> {
        if let Some(close_callback_name) = &req.close_callback {
            Self::register_close_callback(instance, popup_manager, close_callback_name)?;
        }

        if let Some(resize_callback_name) = &req.resize_callback {
            Self::register_resize_direct(
                instance,
                popup_manager,
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
        let surfaces = self.app_state.surfaces_by_name(name);
        let surface = surfaces.first().ok_or_else(|| {
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
