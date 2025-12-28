use crate::dimensions::ScaleFactor;
use crate::errors::{DomainError, Result};
use crate::value_objects::margins::Margins;
use crate::value_objects::output_policy::OutputPolicy;

#[derive(Debug, Clone)]
pub struct LockConfig {
    pub scale_factor: ScaleFactor,
    pub margin: Margins,
    pub namespace: String,
    pub output_policy: OutputPolicy,
}

impl LockConfig {
    #[must_use]
    pub fn new() -> Self {
        Self {
            scale_factor: ScaleFactor::default(),
            margin: Margins::default(),
            namespace: "layer-shika-lock".to_string(),
            output_policy: OutputPolicy::AllOutputs,
        }
    }

    pub fn validate(&self) -> Result<()> {
        let factor = self.scale_factor.value();
        if factor <= 0.0 || !factor.is_finite() {
            return Err(DomainError::InvalidInput {
                message: format!("Lock scale factor must be positive and finite, got {factor}"),
            });
        }
        Ok(())
    }
}

impl Default for LockConfig {
    fn default() -> Self {
        Self::new()
    }
}
