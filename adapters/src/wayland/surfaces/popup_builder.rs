use crate::errors::{LayerShikaError, Result};
use crate::rendering::femtovg::popup_window::PopupWindow;
use layer_shika_domain::value_objects::popup_dimensions::PopupDimensions;
use layer_shika_domain::value_objects::popup_request::{PopupHandle, PopupRequest, PopupSize};
use log::{debug, info};
use slint::ComponentHandle;
use slint_interpreter::{ComponentDefinition, ComponentInstance, Value};
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;
use std::rc::Rc;
use wayland_client::QueueHandle;

use super::popup_manager::{CreatePopupParams, PopupManager};
use super::surface_state::WindowState;

pub struct PopupBuilder<'a> {
    popup_manager: &'a Rc<PopupManager>,
    queue_handle: &'a QueueHandle<WindowState>,
    parent_layer_surface: &'a ZwlrLayerSurfaceV1,
}

impl<'a> PopupBuilder<'a> {
    #[must_use]
    pub const fn new(
        popup_manager: &'a Rc<PopupManager>,
        queue_handle: &'a QueueHandle<WindowState>,
        parent_layer_surface: &'a ZwlrLayerSurfaceV1,
    ) -> Self {
        Self {
            popup_manager,
            queue_handle,
            parent_layer_surface,
        }
    }

    pub fn build(
        &self,
        component_def: &ComponentDefinition,
        request: &PopupRequest,
        serial: u32,
    ) -> Result<PopupHandle> {
        info!(
            "Building popup for component '{}' at position ({}, {}) with mode {:?}",
            request.component,
            request.at.position().0,
            request.at.position().1,
            request.mode
        );

        let dimensions = Self::resolve_dimensions(component_def, &request.size)?;
        dimensions
            .validate()
            .map_err(|e| LayerShikaError::WindowConfiguration {
                message: format!("Invalid popup dimensions: {e}"),
            })?;

        debug!(
            "Resolved popup dimensions: {}x{}",
            dimensions.width(),
            dimensions.height()
        );

        let params = CreatePopupParams {
            last_pointer_serial: serial,
            reference_x: request.at.position().0,
            reference_y: request.at.position().1,
            width: dimensions.width(),
            height: dimensions.height(),
            positioning_mode: request.mode,
        };

        let popup_window = self.popup_manager.create_popup(
            self.queue_handle,
            self.parent_layer_surface,
            params,
        )?;

        let instance = Self::create_component_instance(component_def, &popup_window)?;

        self.setup_popup_callbacks(&instance, &popup_window)?;

        let handle = PopupHandle::new(popup_window.popup_key().ok_or_else(|| {
            LayerShikaError::WindowConfiguration {
                message: "Popup window has no key assigned".to_string(),
            }
        })?);

        info!("Popup built successfully with handle {:?}", handle);

        Ok(handle)
    }

    fn resolve_dimensions(
        component_def: &ComponentDefinition,
        size: &PopupSize,
    ) -> Result<PopupDimensions> {
        match size {
            PopupSize::Fixed { w, h } => {
                debug!("Using fixed popup size: {}x{}", w, h);
                Ok(PopupDimensions::new(*w, *h))
            }
            PopupSize::Content => {
                debug!("Measuring popup dimensions from component content");
                Self::measure_component_dimensions(component_def)
            }
        }
    }

    fn measure_component_dimensions(
        component_def: &ComponentDefinition,
    ) -> Result<PopupDimensions> {
        debug!("Creating temporary component instance to measure dimensions");

        let temp_instance =
            component_def
                .create()
                .map_err(|e| LayerShikaError::WindowConfiguration {
                    message: format!(
                        "Failed to create temporary instance for dimension measurement: {}",
                        e
                    ),
                })?;

        temp_instance
            .show()
            .map_err(|e| LayerShikaError::WindowConfiguration {
                message: format!("Failed to show temporary instance: {}", e),
            })?;

        let width = Self::read_property(&temp_instance, "popup-width", 120.0);
        let height = Self::read_property(&temp_instance, "popup-height", 120.0);

        debug!(
            "Measured dimensions from component properties: {}x{}",
            width, height
        );

        temp_instance
            .hide()
            .map_err(|e| LayerShikaError::WindowConfiguration {
                message: format!("Failed to hide temporary instance: {}", e),
            })?;

        debug!("Hidden temporary instance to release strong reference");

        Ok(PopupDimensions::new(width, height))
    }

    fn read_property(instance: &ComponentInstance, name: &str, default: f32) -> f32 {
        instance
            .get_property(name)
            .ok()
            .and_then(|v| v.try_into().ok())
            .unwrap_or_else(|| {
                debug!(
                    "Property '{}' not found or invalid, using default: {}",
                    name, default
                );
                default
            })
    }

    fn create_component_instance(
        component_def: &ComponentDefinition,
        _popup_window: &Rc<PopupWindow>,
    ) -> Result<ComponentInstance> {
        debug!("Creating popup component instance");

        let instance =
            component_def
                .create()
                .map_err(|e| LayerShikaError::WindowConfiguration {
                    message: format!("Failed to create popup instance: {}", e),
                })?;

        instance
            .show()
            .map_err(|e| LayerShikaError::WindowConfiguration {
                message: format!("Failed to show popup instance: {}", e),
            })?;

        debug!("Popup component instance created and shown successfully");

        Ok(instance)
    }

    fn setup_popup_callbacks(
        &self,
        instance: &ComponentInstance,
        _popup_window: &Rc<PopupWindow>,
    ) -> Result<()> {
        debug!("Setting up popup callbacks");

        let popup_manager = Rc::clone(self.popup_manager);
        instance
            .set_callback("closed", move |_| {
                info!("Popup 'closed' callback triggered");
                popup_manager.close_current_popup();
                Value::Void
            })
            .map_err(|e| LayerShikaError::WindowConfiguration {
                message: format!("Failed to set popup 'closed' callback: {}", e),
            })?;

        debug!("Popup callbacks configured successfully");

        Ok(())
    }
}
