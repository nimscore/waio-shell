use crate::{Error, Result};
use layer_shika_adapters::platform::slint_interpreter::ComponentInstance;
use layer_shika_domain::config::SurfaceConfig;
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::value_objects::handle::SurfaceHandle;
use layer_shika_domain::value_objects::output_handle::OutputHandle;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct SurfaceDefinition {
    pub component: String,
    pub config: SurfaceConfig,
}

#[derive(Clone, Default)]
pub struct SurfaceMetadata {
    pub spawn_order: usize,
    pub creation_timestamp: u64,
}

pub struct SurfaceEntry {
    pub handle: SurfaceHandle,
    pub name: String,
    pub component: String,
    pub definition: SurfaceDefinition,
    pub output_instances: HashMap<OutputHandle, Rc<ComponentInstance>>,
    pub metadata: SurfaceMetadata,
}

impl SurfaceEntry {
    pub fn new(handle: SurfaceHandle, name: String, definition: SurfaceDefinition) -> Self {
        let component = definition.component.clone();
        Self {
            handle,
            name,
            component,
            definition,
            output_instances: HashMap::new(),
            metadata: SurfaceMetadata::default(),
        }
    }

    pub fn add_output_instance(&mut self, output: OutputHandle, instance: Rc<ComponentInstance>) {
        self.output_instances.insert(output, instance);
    }

    pub fn remove_output_instance(
        &mut self,
        output: OutputHandle,
    ) -> Option<Rc<ComponentInstance>> {
        self.output_instances.remove(&output)
    }

    pub fn get_output_instance(&self, output: OutputHandle) -> Option<&Rc<ComponentInstance>> {
        self.output_instances.get(&output)
    }

    pub fn outputs(&self) -> Vec<OutputHandle> {
        self.output_instances.keys().copied().collect()
    }
}

pub struct SurfaceRegistry {
    entries: HashMap<SurfaceHandle, SurfaceEntry>,
    by_name: HashMap<String, SurfaceHandle>,
    by_component: HashMap<String, Vec<SurfaceHandle>>,
    next_spawn_order: usize,
}

impl SurfaceRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            by_name: HashMap::new(),
            by_component: HashMap::new(),
            next_spawn_order: 0,
        }
    }

    pub fn insert(&mut self, mut entry: SurfaceEntry) -> Result<()> {
        if self.by_name.contains_key(&entry.name) {
            return Err(Error::Domain(DomainError::Configuration {
                message: format!("Surface with name '{}' already exists", entry.name),
            }));
        }

        entry.metadata.spawn_order = self.next_spawn_order;
        self.next_spawn_order += 1;

        let handle = entry.handle;
        let name = entry.name.clone();
        let component = entry.component.clone();

        self.by_name.insert(name, handle);

        self.by_component.entry(component).or_default().push(handle);

        self.entries.insert(handle, entry);

        Ok(())
    }

    pub fn remove(&mut self, handle: SurfaceHandle) -> Option<SurfaceEntry> {
        let entry = self.entries.remove(&handle)?;

        self.by_name.remove(&entry.name);

        if let Some(handles) = self.by_component.get_mut(&entry.component) {
            handles.retain(|&h| h != handle);
            if handles.is_empty() {
                self.by_component.remove(&entry.component);
            }
        }

        Some(entry)
    }

    pub fn get(&self, handle: SurfaceHandle) -> Option<&SurfaceEntry> {
        self.entries.get(&handle)
    }

    pub fn get_mut(&mut self, handle: SurfaceHandle) -> Option<&mut SurfaceEntry> {
        self.entries.get_mut(&handle)
    }

    pub fn by_name(&self, name: &str) -> Option<&SurfaceEntry> {
        self.by_name.get(name).and_then(|h| self.entries.get(h))
    }

    pub fn by_name_mut(&mut self, name: &str) -> Option<&mut SurfaceEntry> {
        self.by_name.get(name).and_then(|h| self.entries.get_mut(h))
    }

    pub fn handle_by_name(&self, name: &str) -> Option<SurfaceHandle> {
        self.by_name.get(name).copied()
    }

    pub fn name_by_handle(&self, handle: SurfaceHandle) -> Option<&str> {
        self.entries.get(&handle).map(|e| e.name.as_str())
    }

    pub fn by_component(&self, component: &str) -> Vec<&SurfaceEntry> {
        self.by_component
            .get(component)
            .map(|handles| handles.iter().filter_map(|h| self.entries.get(h)).collect())
            .unwrap_or_default()
    }

    pub fn all(&self) -> impl Iterator<Item = &SurfaceEntry> {
        self.entries.values()
    }

    pub fn all_mut(&mut self) -> impl Iterator<Item = &mut SurfaceEntry> {
        self.entries.values_mut()
    }

    pub fn handles(&self) -> impl Iterator<Item = SurfaceHandle> + '_ {
        self.entries.keys().copied()
    }

    pub fn outputs_for_surface(&self, handle: SurfaceHandle) -> Vec<OutputHandle> {
        self.entries
            .get(&handle)
            .map(SurfaceEntry::outputs)
            .unwrap_or_default()
    }

    pub fn surface_names(&self) -> Vec<&str> {
        self.by_name.keys().map(String::as_str).collect()
    }

    pub fn component_names(&self) -> Vec<&str> {
        self.by_component.keys().map(String::as_str).collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, handle: SurfaceHandle) -> bool {
        self.entries.contains_key(&handle)
    }

    pub fn contains_name(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }
}

impl Default for SurfaceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
