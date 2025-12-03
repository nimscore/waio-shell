use crate::value_objects::output_info::OutputInfo;
use std::fmt;
use std::rc::Rc;

type OutputFilter = Rc<dyn Fn(&OutputInfo) -> bool>;

#[derive(Clone)]
pub enum OutputPolicy {
    AllOutputs,
    PrimaryOnly,
    Custom(OutputFilter),
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
        Self::Custom(Rc::new(filter))
    }
}

impl Default for OutputPolicy {
    fn default() -> Self {
        Self::AllOutputs
    }
}

impl fmt::Debug for OutputPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputPolicy::AllOutputs => write!(f, "OutputPolicy::AllOutputs"),
            OutputPolicy::PrimaryOnly => write!(f, "OutputPolicy::PrimaryOnly"),
            OutputPolicy::Custom(_) => write!(f, "OutputPolicy::Custom(<filter>)"),
        }
    }
}
