use crate::errors::{LayerShikaError, Result};
use layer_shika_domain::value_objects::output_handle::OutputHandle;
use layer_shika_domain::value_objects::output_info::OutputInfo;
use slint_interpreter::{ComponentInstance, Value};
use std::rc::Rc;

pub type LockCallbackHandler = Rc<dyn Fn(&[Value]) -> Value>;
pub type OutputFilter = Rc<
    dyn Fn(
        &str,
        OutputHandle,
        Option<&OutputInfo>,
        Option<OutputHandle>,
        Option<OutputHandle>,
    ) -> bool,
>;

#[derive(Clone)]
pub struct LockCallback {
    name: String,
    handler: LockCallbackHandler,
    filter: Option<OutputFilter>,
}

impl LockCallback {
    pub fn new(name: impl Into<String>, handler: LockCallbackHandler) -> Self {
        Self {
            name: name.into(),
            handler,
            filter: None,
        }
    }

    pub fn with_filter(
        name: impl Into<String>,
        handler: LockCallbackHandler,
        filter: OutputFilter,
    ) -> Self {
        Self {
            name: name.into(),
            handler,
            filter: Some(filter),
        }
    }

    pub fn should_apply(
        &self,
        component_name: &str,
        output_handle: OutputHandle,
        output_info: Option<&OutputInfo>,
        primary_handle: Option<OutputHandle>,
        active_handle: Option<OutputHandle>,
    ) -> bool {
        self.filter.as_ref().map_or_else(
            || true,
            |f| {
                f(
                    component_name,
                    output_handle,
                    output_info,
                    primary_handle,
                    active_handle,
                )
            },
        )
    }

    pub fn apply_to(&self, component: &ComponentInstance) -> Result<()> {
        let handler = Rc::clone(&self.handler);
        component
            .set_callback(&self.name, move |args| handler(args))
            .map_err(|e| LayerShikaError::InvalidInput {
                message: format!("Failed to register callback '{}': {e}", self.name),
            })
    }

    pub const fn name(&self) -> &String {
        &self.name
    }
}
