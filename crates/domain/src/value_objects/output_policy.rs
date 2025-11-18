use crate::value_objects::output_info::OutputInfo;
use std::fmt;

pub enum OutputPolicy {
    AllOutputs,
    PrimaryOnly,
    Custom(Box<dyn Fn(&OutputInfo) -> bool>),
}

impl OutputPolicy {
    pub fn should_render(&self, info: &OutputInfo) -> bool {
        match self {
            OutputPolicy::AllOutputs => true,
            OutputPolicy::PrimaryOnly => info.is_primary(),
            OutputPolicy::Custom(filter) => filter(info),
        }
    }

    pub fn primary_only() -> Self {
        Self::PrimaryOnly
    }

    pub fn all_outputs() -> Self {
        Self::AllOutputs
    }

    pub fn custom<F>(filter: F) -> Self
    where
        F: Fn(&OutputInfo) -> bool + 'static,
    {
        Self::Custom(Box::new(filter))
    }
}

impl Default for OutputPolicy {
    fn default() -> Self {
        Self::AllOutputs
    }
}

impl Clone for OutputPolicy {
    fn clone(&self) -> Self {
        match self {
            OutputPolicy::AllOutputs | OutputPolicy::Custom(_) => OutputPolicy::AllOutputs,
            OutputPolicy::PrimaryOnly => OutputPolicy::PrimaryOnly,
        }
    }
}

impl fmt::Debug for OutputPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputPolicy::AllOutputs => write!(f, "OutputPolicy::AllOutputs"),
            OutputPolicy::PrimaryOnly => write!(f, "OutputPolicy::PrimaryOnly"),
            OutputPolicy::Custom(_) => write!(f, "OutputPolicy::Custom(<function>)"),
        }
    }
}
