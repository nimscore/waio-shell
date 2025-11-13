use crate::errors::DomainError;

pub trait WindowingSystemPort {
    fn run(&mut self) -> Result<(), DomainError>;
}

pub trait RuntimeStatePort {
    fn render_frame_if_dirty(&self) -> Result<(), DomainError>;
}
