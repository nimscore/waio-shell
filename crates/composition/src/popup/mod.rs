use crate::popup_builder::PopupBuilder;
use crate::system::{PopupCommand, ShellCommand, ShellControl};
use crate::{Error, Result};
use layer_shika_adapters::platform::calloop::channel;
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::value_objects::handle::PopupHandle;
use layer_shika_domain::value_objects::popup_config::PopupConfig;

#[derive(Clone)]
pub struct PopupShell {
    sender: channel::Sender<ShellCommand>,
}

impl PopupShell {
    #[must_use]
    pub const fn new(sender: channel::Sender<ShellCommand>) -> Self {
        Self { sender }
    }

    #[must_use]
    pub fn builder(&self, component: impl Into<String>) -> PopupBuilder {
        PopupBuilder::new(component).with_shell(self.clone())
    }

    pub fn show(&self, config: PopupConfig) -> Result<PopupHandle> {
        let handle = PopupHandle::new();
        self.sender
            .send(ShellCommand::Popup(PopupCommand::Show { handle, config }))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send popup show command: channel closed".to_string(),
                })
            })?;
        Ok(handle)
    }

    pub fn close(&self, handle: PopupHandle) -> Result<()> {
        ShellControl::new(self.sender.clone()).close_popup(handle)
    }

    pub fn resize_fixed(&self, handle: PopupHandle, width: f32, height: f32) -> Result<()> {
        ShellControl::new(self.sender.clone()).resize_popup(handle, width, height)
    }
}
