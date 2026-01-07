// TODO: Maybe refactor to reuse the layer shell selector

use crate::{
    Error,
    selector::{Selector, SurfaceInfo},
    slint_interpreter::{ComponentInstance, Value},
};
use layer_shika_domain::errors::DomainError;

/// A selection of session lock surfaces matching a selector
///
/// Provides methods to interact with all matching lock surfaces at once.
/// Created via `Shell::select_lock()`.
pub struct LockSelection<'a> {
    shell: &'a crate::Shell,
    selector: Selector,
}

impl<'a> LockSelection<'a> {
    pub(crate) fn new(shell: &'a crate::Shell, selector: Selector) -> Self {
        Self { shell, selector }
    }

    /// Registers a callback handler for all matching lock surfaces
    ///
    /// Handler receives a `CallbackContext` with surface identity and shell control.
    /// Callbacks are stored and applied when the lock is activated, and automatically
    /// applied to new surfaces when outputs are hotplugged during an active lock.
    pub fn on_callback<F, R>(&mut self, callback_name: &str, handler: F) -> &mut Self
    where
        F: Fn(crate::CallbackContext) -> R + Clone + 'static,
        R: crate::IntoValue,
    {
        self.shell
            .on_lock_internal(&self.selector, callback_name, handler);
        self
    }

    /// Registers a callback handler that receives arguments for all matching lock surfaces
    pub fn on_callback_with_args<F, R>(&mut self, callback_name: &str, handler: F) -> &mut Self
    where
        F: Fn(&[Value], crate::CallbackContext) -> R + Clone + 'static,
        R: crate::IntoValue,
    {
        self.shell
            .on_lock_with_args_internal(&self.selector, callback_name, handler);
        self
    }

    /// Executes a function with each matching lock component instance
    ///
    /// Returns immediately if no lock surfaces are active. During activation,
    /// this iterates over all lock component instances matching the selector.
    pub fn with_component<F>(&self, mut f: F)
    where
        F: FnMut(&ComponentInstance),
    {
        self.shell
            .with_selected_lock(&self.selector, |_, component| {
                f(component);
            });
    }

    /// Sets a property value on all matching lock surfaces
    ///
    /// If the lock is inactive, this operation succeeds silently with no effect.
    /// If the lock is active, the property is set on all matching component instances.
    pub fn set_property(&self, name: &str, value: &Value) -> Result<(), Error> {
        let mut result = Ok(());
        self.shell
            .with_selected_lock(&self.selector, |_, component| {
                if let Err(e) = component.set_property(name, value.clone()) {
                    log::error!("Failed to set property '{}' on lock surface: {}", name, e);
                    result = Err(Error::Domain(DomainError::Configuration {
                        message: format!("Failed to set property '{}': {}", name, e),
                    }));
                }
            });
        result
    }

    /// Gets property values from all matching lock surfaces
    ///
    /// Returns an empty vector if the lock is inactive.
    pub fn get_property(&self, name: &str) -> Result<Vec<Value>, Error> {
        let mut values = Vec::new();
        let mut result = Ok(());
        self.shell
            .with_selected_lock(&self.selector, |_, component| {
                match component.get_property(name) {
                    Ok(value) => values.push(value),
                    Err(e) => {
                        log::error!("Failed to get property '{}' from lock surface: {}", name, e);
                        result = Err(Error::Domain(DomainError::Configuration {
                            message: format!("Failed to get property '{}': {}", name, e),
                        }));
                    }
                }
            });
        result.map(|()| values)
    }

    /// Returns the number of lock surfaces matching the selector
    ///
    /// Returns 0 if the lock is inactive.
    pub fn count(&self) -> usize {
        self.shell.count_selected_lock(&self.selector)
    }

    /// Checks if no lock surfaces match the selector
    ///
    /// Returns true if the lock is inactive or no surfaces match the selector.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Returns information about all matching lock surfaces
    ///
    /// Returns an empty vector if the lock is inactive.
    pub fn info(&self) -> Vec<SurfaceInfo> {
        self.shell.get_selected_lock_info(&self.selector)
    }
}
