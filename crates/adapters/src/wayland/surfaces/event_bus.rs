use super::window_events::WindowStateEvent;
use std::cell::RefCell;
use std::rc::Rc;

type EventHandlerFn = Box<dyn Fn(&WindowStateEvent)>;

#[derive(Clone)]
pub struct EventBus {
    handlers: Rc<RefCell<Vec<EventHandlerFn>>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn publish(&self, event: &WindowStateEvent) {
        let handlers = self.handlers.borrow();
        for handler in handlers.iter() {
            handler(event);
        }
    }
}
