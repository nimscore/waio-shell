use crate::system::PopupCommand;
use crate::{Error, Result};
use layer_shika_adapters::PopupManager;
use layer_shika_adapters::platform::calloop::channel;
use layer_shika_adapters::platform::slint::SharedString;
use layer_shika_adapters::platform::slint_interpreter::{ComponentInstance, Value};
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::value_objects::popup_positioning_mode::PopupPositioningMode;
use layer_shika_domain::value_objects::popup_request::{
    PopupAt, PopupHandle, PopupRequest, PopupSize,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct SlintCallbackNames;

impl SlintCallbackNames {
    pub const SHOW_POPUP: &'static str = "show_popup";
    pub const CHANGE_POPUP_SIZE: &'static str = "change_popup_size";
    pub const SET_POPUP_POSITIONING_MODE: &'static str = "set_popup_positioning_mode";
    pub const POPUP_CLOSED: &'static str = "closed";
}

pub struct SlintCallbackContract {
    popup_positioning_mode: Rc<RefCell<PopupPositioningMode>>,
    popup_command_sender: channel::Sender<PopupCommand>,
}

impl SlintCallbackContract {
    #[must_use]
    pub fn new(
        popup_positioning_mode: Rc<RefCell<PopupPositioningMode>>,
        popup_command_sender: channel::Sender<PopupCommand>,
    ) -> Self {
        Self {
            popup_positioning_mode,
            popup_command_sender,
        }
    }

    pub fn register_on_main_component(&self, component_instance: &ComponentInstance) -> Result<()> {
        self.register_set_popup_positioning_mode_callback(component_instance)?;
        self.register_show_popup_callback(component_instance)?;
        Ok(())
    }

    pub fn register_on_popup_component(
        instance: &ComponentInstance,
        popup_manager: &Rc<PopupManager>,
        resize_sender: Option<channel::Sender<PopupCommand>>,
        popup_key_cell: &Rc<Cell<usize>>,
    ) -> Result<()> {
        Self::register_popup_closed_callback(instance, popup_manager)?;
        Self::register_change_popup_size_callback(
            instance,
            popup_manager,
            resize_sender,
            popup_key_cell,
        );
        Ok(())
    }

    fn register_set_popup_positioning_mode_callback(
        &self,
        component_instance: &ComponentInstance,
    ) -> Result<()> {
        let popup_mode_clone = Rc::clone(&self.popup_positioning_mode);
        component_instance
            .set_callback(
                SlintCallbackNames::SET_POPUP_POSITIONING_MODE,
                move |args| {
                    let center_x: bool = args
                        .first()
                        .and_then(|v| v.clone().try_into().ok())
                        .unwrap_or(false);
                    let center_y: bool = args
                        .get(1)
                        .and_then(|v| v.clone().try_into().ok())
                        .unwrap_or(false);

                    let mode = PopupPositioningMode::from_flags(center_x, center_y);
                    *popup_mode_clone.borrow_mut() = mode;
                    log::info!(
                        "Popup positioning mode set to: {:?} (center_x: {}, center_y: {})",
                        mode,
                        center_x,
                        center_y
                    );
                    Value::Void
                },
            )
            .map_err(|e| {
                Error::Domain(DomainError::Configuration {
                    message: format!(
                        "Failed to register {} callback: {}",
                        SlintCallbackNames::SET_POPUP_POSITIONING_MODE,
                        e
                    ),
                })
            })
    }

    fn register_show_popup_callback(&self, component_instance: &ComponentInstance) -> Result<()> {
        let sender = self.popup_command_sender.clone();
        let popup_mode_for_callback = Rc::clone(&self.popup_positioning_mode);

        component_instance
            .set_callback(SlintCallbackNames::SHOW_POPUP, move |args| {
                let component_name: SharedString = args
                    .first()
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or_else(|| SharedString::from(""));

                if component_name.is_empty() {
                    log::error!(
                        "{} called without component name",
                        SlintCallbackNames::SHOW_POPUP
                    );
                    return Value::Void;
                }

                let x: f32 = args
                    .get(1)
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(0.0);
                let y: f32 = args
                    .get(2)
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(0.0);

                let mode = *popup_mode_for_callback.borrow();

                let request = PopupRequest::builder(component_name.to_string())
                    .at(PopupAt::absolute(x, y))
                    .size(PopupSize::content())
                    .mode(mode)
                    .build();

                if sender.send(PopupCommand::Show(request)).is_err() {
                    log::error!("Failed to send popup show command through channel");
                }
                Value::Void
            })
            .map_err(|e| {
                Error::Domain(DomainError::Configuration {
                    message: format!(
                        "Failed to register {} callback: {}",
                        SlintCallbackNames::SHOW_POPUP,
                        e
                    ),
                })
            })
    }

    fn register_popup_closed_callback(
        instance: &ComponentInstance,
        popup_manager: &Rc<PopupManager>,
    ) -> Result<()> {
        let popup_manager_weak = Rc::downgrade(popup_manager);
        instance
            .set_callback(SlintCallbackNames::POPUP_CLOSED, move |_| {
                if let Some(popup_manager) = popup_manager_weak.upgrade() {
                    popup_manager.close_current_popup();
                }
                Value::Void
            })
            .map_err(|e| {
                Error::Domain(DomainError::Configuration {
                    message: format!(
                        "Failed to set {} callback: {}",
                        SlintCallbackNames::POPUP_CLOSED,
                        e
                    ),
                })
            })
    }

    fn register_change_popup_size_callback(
        instance: &ComponentInstance,
        popup_manager: &Rc<PopupManager>,
        resize_sender: Option<channel::Sender<PopupCommand>>,
        popup_key_cell: &Rc<Cell<usize>>,
    ) {
        let result = if let Some(sender) = resize_sender {
            let key_cell = Rc::clone(popup_key_cell);
            instance.set_callback(SlintCallbackNames::CHANGE_POPUP_SIZE, move |args| {
                let width: f32 = args
                    .first()
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(200.0);
                let height: f32 = args
                    .get(1)
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(150.0);

                let popup_key = key_cell.get();
                log::info!(
                    "{} callback invoked: {}x{} for key {}",
                    SlintCallbackNames::CHANGE_POPUP_SIZE,
                    width,
                    height,
                    popup_key
                );

                if sender
                    .send(PopupCommand::Resize {
                        handle: PopupHandle::new(popup_key),
                        width,
                        height,
                    })
                    .is_err()
                {
                    log::error!("Failed to send popup resize command through channel");
                }
                Value::Void
            })
        } else {
            let popup_manager_for_resize = Rc::downgrade(popup_manager);
            let key_cell = Rc::clone(popup_key_cell);
            instance.set_callback(SlintCallbackNames::CHANGE_POPUP_SIZE, move |args| {
                let width: f32 = args
                    .first()
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(200.0);
                let height: f32 = args
                    .get(1)
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(150.0);

                let popup_key = key_cell.get();
                log::info!(
                    "{} callback invoked: {}x{} for key {}",
                    SlintCallbackNames::CHANGE_POPUP_SIZE,
                    width,
                    height,
                    popup_key
                );

                if let Some(popup_window) = popup_manager_for_resize
                    .upgrade()
                    .and_then(|mgr| mgr.get_popup_window(popup_key))
                {
                    popup_window.request_resize(width, height);
                }
                Value::Void
            })
        };

        if let Err(e) = result {
            log::warn!(
                "Failed to set {} callback: {}",
                SlintCallbackNames::CHANGE_POPUP_SIZE,
                e
            );
        } else {
            log::info!(
                "{} callback registered successfully",
                SlintCallbackNames::CHANGE_POPUP_SIZE
            );
        }
    }
}
