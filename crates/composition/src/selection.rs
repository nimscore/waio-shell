use crate::{
    Error, LayerSurfaceHandle, Shell,
    selector::{Selector, SurfaceInfo},
    slint_interpreter::{ComponentInstance, Value},
};
use layer_shika_domain::errors::DomainError;

/// A selection of surfaces matching a selector
///
/// Provides methods to interact with all matching surfaces at once.
/// Created via `Shell::select()`.
pub struct Selection<'a> {
    shell: &'a Shell,
    selector: Selector,
}

impl<'a> Selection<'a> {
    pub(crate) fn new(shell: &'a Shell, selector: Selector) -> Self {
        Self { shell, selector }
    }

    /// Registers a callback handler for all matching surfaces
    ///
    /// Handler receives a `CallbackContext` with surface identity and shell control.
    pub fn on_callback<F, R>(&mut self, callback_name: &str, handler: F) -> &mut Self
    where
        F: Fn(crate::CallbackContext) -> R + Clone + 'static,
        R: crate::IntoValue,
    {
        self.shell
            .on_internal(&self.selector, callback_name, handler);
        self
    }

    /// Registers a callback handler that receives arguments for all matching surfaces
    pub fn on_callback_with_args<F, R>(&mut self, callback_name: &str, handler: F) -> &mut Self
    where
        F: Fn(&[Value], crate::CallbackContext) -> R + Clone + 'static,
        R: crate::IntoValue,
    {
        self.shell
            .on_with_args_internal(&self.selector, callback_name, handler);
        self
    }

    /// Executes a function with each matching component instance
    pub fn with_component<F>(&self, mut f: F)
    where
        F: FnMut(&ComponentInstance),
    {
        self.shell.with_selected(&self.selector, |_, component| {
            f(component);
        });
    }

    /// Sets a property value on all matching surfaces
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

    /// Gets property values from all matching surfaces
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

    /// Executes a configuration function with component and surface handle for matching surfaces
    pub fn configure<F>(&self, mut f: F)
    where
        F: FnMut(&ComponentInstance, LayerSurfaceHandle<'_>),
    {
        self.shell
            .configure_selected(&self.selector, |component, handle| {
                f(component, handle);
            });
    }

    /// Returns the number of surfaces matching the selector
    pub fn count(&self) -> usize {
        self.shell.count_selected(&self.selector)
    }

    /// Checks if no surfaces match the selector
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Returns information about all matching surfaces
    pub fn info(&self) -> Vec<SurfaceInfo> {
        self.shell.get_selected_info(&self.selector)
    }
}
