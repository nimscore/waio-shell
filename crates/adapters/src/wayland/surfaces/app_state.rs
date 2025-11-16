use super::surface_state::WindowState;
use super::event_context::SharedPointerSerial;
use crate::wayland::managed_proxies::ManagedWlPointer;
use crate::wayland::outputs::OutputKey;
use std::collections::HashMap;
use std::rc::Rc;
use wayland_client::backend::ObjectId;
use wayland_client::Proxy;

pub type PerOutputWindow = WindowState;

pub struct AppState {
    outputs: HashMap<OutputKey, PerOutputWindow>,
    surface_to_output: HashMap<ObjectId, OutputKey>,
    output_to_key: HashMap<ObjectId, OutputKey>,
    _pointer: ManagedWlPointer,
    shared_pointer_serial: Rc<SharedPointerSerial>,
    active_output: Option<OutputKey>,
}

impl AppState {
    pub fn new(pointer: ManagedWlPointer, shared_serial: Rc<SharedPointerSerial>) -> Self {
        Self {
            outputs: HashMap::new(),
            surface_to_output: HashMap::new(),
            output_to_key: HashMap::new(),
            _pointer: pointer,
            shared_pointer_serial: shared_serial,
            active_output: None,
        }
    }

    pub fn add_output(
        &mut self,
        output_id: ObjectId,
        main_surface_id: ObjectId,
        window: PerOutputWindow,
    ) {
        let key = OutputKey::new(output_id.clone());
        self.output_to_key.insert(output_id, key.clone());
        self.surface_to_output
            .insert(main_surface_id, key.clone());
        self.outputs.insert(key, window);
    }

    pub fn get_output_by_key(&self, key: &OutputKey) -> Option<&PerOutputWindow> {
        self.outputs.get(key)
    }

    pub fn get_output_by_key_mut(&mut self, key: &OutputKey) -> Option<&mut PerOutputWindow> {
        self.outputs.get_mut(key)
    }

    pub fn get_output_by_output_id(&self, output_id: &ObjectId) -> Option<&PerOutputWindow> {
        self.output_to_key
            .get(output_id)
            .and_then(|key| self.outputs.get(key))
    }

    pub fn get_output_by_output_id_mut(
        &mut self,
        output_id: &ObjectId,
    ) -> Option<&mut PerOutputWindow> {
        self.output_to_key
            .get(output_id)
            .and_then(|key| self.outputs.get_mut(key))
    }

    pub fn get_output_by_surface(&self, surface_id: &ObjectId) -> Option<&PerOutputWindow> {
        self.surface_to_output
            .get(surface_id)
            .and_then(|key| self.outputs.get(key))
    }

    pub fn get_output_by_surface_mut(
        &mut self,
        surface_id: &ObjectId,
    ) -> Option<&mut PerOutputWindow> {
        self.surface_to_output
            .get(surface_id)
            .and_then(|key| self.outputs.get_mut(key))
    }

    pub fn get_output_by_layer_surface_mut(
        &mut self,
        layer_surface_id: &ObjectId,
    ) -> Option<&mut PerOutputWindow> {
        self.outputs.values_mut().find(|window| {
            window.layer_surface().as_ref().id() == *layer_surface_id
        })
    }

    pub fn get_key_by_surface(&self, surface_id: &ObjectId) -> Option<&OutputKey> {
        self.surface_to_output.get(surface_id)
    }

    pub fn get_key_by_output_id(&self, output_id: &ObjectId) -> Option<&OutputKey> {
        self.output_to_key.get(output_id)
    }

    pub fn register_popup_surface(&mut self, popup_surface_id: ObjectId, output_key: OutputKey) {
        self.surface_to_output.insert(popup_surface_id, output_key);
    }

    pub fn set_active_output(&mut self, key: Option<OutputKey>) {
        self.active_output = key;
    }

    pub const fn active_output(&self) -> Option<&OutputKey> {
        self.active_output.as_ref()
    }

    pub fn primary_output(&self) -> Option<&PerOutputWindow> {
        self.outputs.values().next()
    }

    pub fn all_outputs(&self) -> impl Iterator<Item = &PerOutputWindow> {
        self.outputs.values()
    }

    pub fn all_outputs_mut(&mut self) -> impl Iterator<Item = &mut PerOutputWindow> {
        self.outputs.values_mut()
    }

    pub const fn shared_pointer_serial(&self) -> &Rc<SharedPointerSerial> {
        &self.shared_pointer_serial
    }

    pub fn find_output_by_popup(&self, popup_surface_id: &ObjectId) -> Option<&PerOutputWindow> {
        self.outputs.values().find(|window| {
            window
                .popup_manager()
                .as_ref()
                .and_then(|pm| pm.find_by_surface(popup_surface_id))
                .is_some()
        })
    }

    pub fn find_output_by_popup_mut(
        &mut self,
        popup_surface_id: &ObjectId,
    ) -> Option<&mut PerOutputWindow> {
        self.outputs.values_mut().find(|window| {
            window
                .popup_manager()
                .as_ref()
                .and_then(|pm| pm.find_by_surface(popup_surface_id))
                .is_some()
        })
    }

    pub fn get_key_by_popup(&self, popup_surface_id: &ObjectId) -> Option<OutputKey> {
        self.outputs.iter().find_map(|(key, window)| {
            window
                .popup_manager()
                .as_ref()
                .and_then(|pm| pm.find_by_surface(popup_surface_id))
                .map(|_| key.clone())
        })
    }
}
