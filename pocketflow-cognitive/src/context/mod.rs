//! Context extensions for cognitive operations.
//!
//! This module provides extension traits for the Context type to add
//! cognitive memory and reasoning capabilities.

use pocketflow_core::context::Context;

use crate::{Result, memory::CognitiveMemory};

/// Extension trait for adding cognitive capabilities to Context
pub trait CognitiveContextExt {
    /// Get the cognitive memory from context
    fn get_cognitive_memory(&self) -> Result<Option<CognitiveMemory>>;

    /// Set the cognitive memory in context
    fn set_cognitive_memory(&mut self, memory: CognitiveMemory) -> Result<()>;

    /// Get or create cognitive memory
    fn get_or_create_cognitive_memory(&mut self) -> Result<CognitiveMemory>;

    /// Add a thought to working memory
    fn add_thought(&mut self, thought: String) -> Result<()>;

    /// Get recent thoughts from working memory
    fn get_recent_thoughts(&self) -> Result<Vec<String>>;

    /// Set the current reasoning focus
    fn set_reasoning_focus(&mut self, focus: String) -> Result<()>;

    /// Get the current reasoning focus
    fn get_reasoning_focus(&self) -> Result<Option<String>>;

    /// Store reasoning trace
    fn store_reasoning_trace(&mut self, trace: serde_json::Value) -> Result<()>;

    /// Retrieve reasoning trace
    fn get_reasoning_trace(&self) -> Result<Option<serde_json::Value>>;

    /// Check if context has cognitive memory
    fn has_cognitive_memory(&self) -> bool;
}

impl CognitiveContextExt for Context {
    fn get_cognitive_memory(&self) -> Result<Option<CognitiveMemory>> {
        self.get_json("cognitive_memory")
    }

    fn set_cognitive_memory(&mut self, memory: CognitiveMemory) -> Result<()> {
        self.set("cognitive_memory", &memory)
    }

    fn get_or_create_cognitive_memory(&mut self) -> Result<CognitiveMemory> {
        match self.get_cognitive_memory()? {
            Some(memory) => Ok(memory),
            None => {
                let memory = CognitiveMemory::new();
                self.set_cognitive_memory(memory.clone())?;
                Ok(memory)
            }
        }
    }

    fn add_thought(&mut self, thought: String) -> Result<()> {
        let mut memory = self.get_or_create_cognitive_memory()?;
        memory.working_memory.add_thought(thought);
        self.set_cognitive_memory(memory)
    }

    fn get_recent_thoughts(&self) -> Result<Vec<String>> {
        match self.get_cognitive_memory()? {
            Some(memory) => Ok(memory.working_memory.get_recent_thoughts().to_vec()),
            None => Ok(Vec::new()),
        }
    }

    fn set_reasoning_focus(&mut self, focus: String) -> Result<()> {
        let mut memory = self.get_or_create_cognitive_memory()?;
        memory.working_memory.set_focus(focus);
        self.set_cognitive_memory(memory)
    }

    fn get_reasoning_focus(&self) -> Result<Option<String>> {
        match self.get_cognitive_memory()? {
            Some(memory) => Ok(memory.working_memory.get_focus().cloned()),
            None => Ok(None),
        }
    }

    fn store_reasoning_trace(&mut self, trace: serde_json::Value) -> Result<()> {
        self.set("reasoning_trace", &trace)
    }

    fn get_reasoning_trace(&self) -> Result<Option<serde_json::Value>> {
        self.get_json("reasoning_trace")
    }

    fn has_cognitive_memory(&self) -> bool {
        self.contains_json("cognitive_memory")
    }
}
