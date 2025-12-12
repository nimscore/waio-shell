use crate::{
    Error, LayerSurfaceHandle, Shell,
    selector::{Selector, SurfaceInfo},
    slint_interpreter::{ComponentInstance, Value},
};
use layer_shika_domain::errors::DomainError;

/// A selection of surfaces matching a selector
///
/// Provides methods to interact with all matching surfaces at once, such as
/// setting up callbacks, modifying properties, or accessing component instances.
/// Created via `Shell::select()`.
pub struct Selection<'a> {
    shell: &'a Shell,
    selector: Selector,
}

impl<'a> Selection<'a> {
    pub(crate) fn new(shell: &'a Shell, selector: Selector) -> Self {
        Self { shell, selector }
    }

    pub fn on_callback<F, R>(&mut self, callback_name: &str, handler: F) -> &mut Self
    where
        F: Fn(crate::CallbackContext) -> R + Clone + 'static,
        R: crate::IntoValue,
    {
        self.shell
            .on_internal(&self.selector, callback_name, handler);
        self
    }

    pub fn on_callback_with_args<F, R>(&mut self, callback_name: &str, handler: F) -> &mut Self
    where
        F: Fn(&[Value], crate::CallbackContext) -> R + Clone + 'static,
        R: crate::IntoValue,
    {
        self.shell
            .on_with_args_internal(&self.selector, callback_name, handler);
        self
    }

    pub fn with_component<F>(&self, mut f: F)
    where
        F: FnMut(&ComponentInstance),
    {
        self.shell.with_selected(&self.selector, |_, component| {
            f(component);
        });
    }

    pub fn set_property(&self, name: &str, value: &Value) -> Result<(), Error> {
        let mut result = Ok(());
        self.shell.with_selected(&self.selector, |_, component| {
            if let Err(e) = component.set_property(name, value.clone()) {
                log::error!("Failed to set property '{}': {}", name, e);
                result = Err(Error::Domain(DomainError::Configuration {
                    message: format!("Failed to set property '{}': {}", name, e),
                }));
            }
        });
        result
    }

    pub fn get_property(&self, name: &str) -> Result<Vec<Value>, Error> {
        let mut values = Vec::new();
        let mut result = Ok(());
        self.shell.with_selected(&self.selector, |_, component| {
            match component.get_property(name) {
                Ok(value) => values.push(value),
                Err(e) => {
                    log::error!("Failed to get property '{}': {}", name, e);
                    result = Err(Error::Domain(DomainError::Configuration {
                        message: format!("Failed to get property '{}': {}", name, e),
                    }));
                }
            }
        });
        result.map(|()| values)
    }

    pub fn configure<F>(&self, mut f: F)
    where
        F: FnMut(&ComponentInstance, LayerSurfaceHandle<'_>),
    {
        self.shell
            .configure_selected(&self.selector, |component, handle| {
                f(component, handle);
            });
    }

    pub fn count(&self) -> usize {
        self.shell.count_selected(&self.selector)
    }

    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    pub fn info(&self) -> Vec<SurfaceInfo> {
        self.shell.get_selected_info(&self.selector)
    }
}
