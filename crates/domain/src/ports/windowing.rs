use crate::errors::DomainError;

pub trait WindowingSystemPort {
    fn run(&mut self) -> Result<(), DomainError>;
}

pub trait ShellContextPort {
    fn render_frame_if_dirty(&mut self) -> Result<(), DomainError>;
}
