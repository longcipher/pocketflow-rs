//! Context management for PocketFlow workflows.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::error::{FlowError, Result};

/// Type-safe shared context for workflow execution.
///
/// The Context provides a way to store and retrieve data between nodes
/// in a workflow. It supports both typed data access and JSON serialization.
#[derive(Clone, Debug, Default)]
pub struct Context {
    /// Typed data storage
    data: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    /// JSON data storage for serializable data
    json_data: HashMap<String, Value>,
    /// Metadata for the context
    metadata: HashMap<String, Value>,
}

impl Context {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a context from JSON data.
    pub fn from_json(data: HashMap<String, Value>) -> Self {
        Self {
            data: HashMap::new(),
            json_data: data,
            metadata: HashMap::new(),
        }
    }

    /// Insert typed data into the context.
    ///
    /// This provides type-safe storage and retrieval of data.
    pub fn insert<T>(&mut self, value: T) -> Result<()>
    where
        T: Send + Sync + 'static,
    {
        self.data.insert(TypeId::of::<T>(), Arc::new(value));
        Ok(())
    }

    /// Get typed data from the context.
    pub fn get<T>(&self) -> Option<&T>
    where
        T: Send + Sync + 'static,
    {
        self.data
            .get(&TypeId::of::<T>())
            .and_then(|data| data.downcast_ref::<T>())
    }

    /// Remove typed data from the context.
    pub fn remove<T>(&mut self) -> Option<T>
    where
        T: Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        if let Some(data) = self.data.remove(&type_id) {
            // Try to downcast the Arc<dyn Any + Send + Sync> to Arc<T>
            match data.downcast::<T>() {
                Ok(arc_t) => {
                    // Try to unwrap the Arc to get the owned value
                    Arc::try_unwrap(arc_t).ok()
                }
                Err(_) => None, // Downcast failed
            }
        } else {
            None
        }
    }

    /// Check if typed data exists in the context.
    pub fn contains<T>(&self) -> bool
    where
        T: Send + Sync + 'static,
    {
        self.data.contains_key(&TypeId::of::<T>())
    }

    /// Set JSON data by key.
    pub fn set(&mut self, key: impl Into<String>, value: impl Serialize) -> Result<()> {
        let json_value = serde_json::to_value(value)?;
        self.json_data.insert(key.into(), json_value);
        Ok(())
    }

    /// Get JSON data by key and deserialize it.
    pub fn get_json<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        self.json_data
            .get(key)
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
            .map_err(FlowError::from)
    }

    /// Get raw JSON value by key.
    pub fn get_raw(&self, key: &str) -> Option<&Value> {
        self.json_data.get(key)
    }

    /// Remove JSON data by key.
    pub fn remove_json(&mut self, key: &str) -> Option<Value> {
        self.json_data.remove(key)
    }

    /// Check if JSON data exists by key.
    pub fn contains_json(&self, key: &str) -> bool {
        self.json_data.contains_key(key)
    }

    /// Get all JSON data keys.
    pub fn json_keys(&self) -> impl Iterator<Item = &String> {
        self.json_data.keys()
    }

    /// Set metadata.
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Serialize) -> Result<()> {
        let json_value = serde_json::to_value(value)?;
        self.metadata.insert(key.into(), json_value);
        Ok(())
    }

    /// Get metadata.
    pub fn get_metadata<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        self.metadata
            .get(key)
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
            .map_err(FlowError::from)
    }

    /// Get raw metadata value.
    pub fn get_metadata_raw(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }

    /// Merge another context into this one.
    ///
    /// JSON data and metadata from the other context will override
    /// existing values with the same keys.
    pub fn merge(&mut self, other: &Context) {
        // Note: We can't merge typed data safely without knowing the types
        // so we only merge JSON data and metadata
        for (key, value) in &other.json_data {
            self.json_data.insert(key.clone(), value.clone());
        }
        for (key, value) in &other.metadata {
            self.metadata.insert(key.clone(), value.clone());
        }
    }

    /// Clear all data from the context.
    pub fn clear(&mut self) {
        self.data.clear();
        self.json_data.clear();
        self.metadata.clear();
    }

    /// Get the number of items in the context.
    pub fn len(&self) -> usize {
        self.data.len() + self.json_data.len()
    }

    /// Check if the context is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty() && self.json_data.is_empty()
    }

    /// Convert the context to a JSON representation (JSON data only).
    pub fn to_json(&self) -> Result<Value> {
        serde_json::to_value(&self.json_data).map_err(FlowError::from)
    }

    /// Get all JSON data as a HashMap.
    pub fn json_data(&self) -> &HashMap<String, Value> {
        &self.json_data
    }

    /// Get all metadata as a HashMap.
    pub fn metadata(&self) -> &HashMap<String, Value> {
        &self.metadata
    }
}

/// Builder for creating contexts with initial data.
#[derive(Default)]
pub struct ContextBuilder {
    context: Context,
}

impl ContextBuilder {
    /// Create a new context builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert typed data.
    pub fn with<T>(mut self, value: T) -> Result<Self>
    where
        T: Send + Sync + 'static,
    {
        self.context.insert(value)?;
        Ok(self)
    }

    /// Set JSON data.
    pub fn with_json(mut self, key: impl Into<String>, value: impl Serialize) -> Result<Self> {
        self.context.set(key, value)?;
        Ok(self)
    }

    /// Set metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Serialize) -> Result<Self> {
        self.context.set_metadata(key, value)?;
        Ok(self)
    }

    /// Build the context.
    pub fn build(self) -> Context {
        self.context
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestData {
        value: i32,
        name: String,
    }

    #[test]
    fn test_typed_data() {
        let mut context = Context::new();

        // Insert typed data
        context.insert(42i32).unwrap();
        context.insert("hello".to_string()).unwrap();

        // Retrieve typed data
        assert_eq!(context.get::<i32>(), Some(&42));
        assert_eq!(context.get::<String>(), Some(&"hello".to_string()));
        assert_eq!(context.get::<u32>(), None);

        // Check contains
        assert!(context.contains::<i32>());
        assert!(context.contains::<String>());
        assert!(!context.contains::<u32>());

        // Remove typed data
        assert_eq!(context.remove::<i32>(), Some(42));
        assert_eq!(context.get::<i32>(), None);
    }

    #[test]
    fn test_json_data() {
        let mut context = Context::new();

        let test_data = TestData {
            value: 123,
            name: "test".to_string(),
        };

        // Set JSON data
        context.set("test", &test_data).unwrap();
        context.set("number", 456).unwrap();

        // Get JSON data
        let retrieved: TestData = context.get_json("test").unwrap().unwrap();
        assert_eq!(retrieved, test_data);

        let number: i32 = context.get_json("number").unwrap().unwrap();
        assert_eq!(number, 456);

        // Check contains
        assert!(context.contains_json("test"));
        assert!(context.contains_json("number"));
        assert!(!context.contains_json("missing"));

        // Remove JSON data
        assert!(context.remove_json("test").is_some());
        assert!(context.get_json::<TestData>("test").unwrap().is_none());
    }

    #[test]
    fn test_metadata() {
        let mut context = Context::new();

        context.set_metadata("version", "1.0").unwrap();
        context.set_metadata("timestamp", 1234567890u64).unwrap();

        let version: String = context.get_metadata("version").unwrap().unwrap();
        assert_eq!(version, "1.0");

        let timestamp: u64 = context.get_metadata("timestamp").unwrap().unwrap();
        assert_eq!(timestamp, 1234567890);
    }

    #[test]
    fn test_context_builder() {
        let context = ContextBuilder::new()
            .with(42i32)
            .unwrap()
            .with_json("name", "test")
            .unwrap()
            .with_metadata("version", "1.0")
            .unwrap()
            .build();

        assert_eq!(context.get::<i32>(), Some(&42));
        assert_eq!(
            context.get_json::<String>("name").unwrap(),
            Some("test".to_string())
        );
        assert_eq!(
            context.get_metadata::<String>("version").unwrap(),
            Some("1.0".to_string())
        );
    }

    #[test]
    fn test_context_merge() {
        let mut context1 = Context::new();
        context1.set("key1", "value1").unwrap();
        context1.set_metadata("meta1", "metavalue1").unwrap();

        let mut context2 = Context::new();
        context2.set("key2", "value2").unwrap();
        context2.set("key1", "newvalue1").unwrap(); // Should override
        context2.set_metadata("meta2", "metavalue2").unwrap();

        context1.merge(&context2);

        assert_eq!(
            context1.get_json::<String>("key1").unwrap(),
            Some("newvalue1".to_string())
        );
        assert_eq!(
            context1.get_json::<String>("key2").unwrap(),
            Some("value2".to_string())
        );
        assert_eq!(
            context1.get_metadata::<String>("meta1").unwrap(),
            Some("metavalue1".to_string())
        );
        assert_eq!(
            context1.get_metadata::<String>("meta2").unwrap(),
            Some("metavalue2".to_string())
        );
    }
}
