//! Memory systems for cognitive operations.
//!
//! This module provides multi-layered memory systems including working memory,
//! episodic memory, and semantic memory for enhanced context management.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Multi-layered cognitive memory system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveMemory {
    pub working_memory: WorkingMemory,
    pub episodic_memory: EpisodicMemory,
    pub semantic_memory: SemanticMemory,
}

impl CognitiveMemory {
    pub fn new() -> Self {
        Self {
            working_memory: WorkingMemory::new(),
            episodic_memory: EpisodicMemory::new(),
            semantic_memory: SemanticMemory::new(),
        }
    }
}

impl Default for CognitiveMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// Working memory for short-term context and active reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemory {
    current_focus: Option<String>,
    active_thoughts: Vec<String>,
    temporary_data: HashMap<String, serde_json::Value>,
    attention_stack: Vec<String>,
}

impl WorkingMemory {
    pub fn new() -> Self {
        Self {
            current_focus: None,
            active_thoughts: Vec::new(),
            temporary_data: HashMap::new(),
            attention_stack: Vec::new(),
        }
    }

    pub fn set_focus(&mut self, focus: String) {
        if let Some(current) = &self.current_focus {
            self.attention_stack.push(current.clone());
        }
        self.current_focus = Some(focus);
    }

    pub fn get_focus(&self) -> Option<&String> {
        self.current_focus.as_ref()
    }

    pub fn add_thought(&mut self, thought: String) {
        self.active_thoughts.push(thought);
        // Keep only recent thoughts to prevent memory bloat
        if self.active_thoughts.len() > 10 {
            self.active_thoughts.remove(0);
        }
    }

    pub fn get_recent_thoughts(&self) -> &[String] {
        &self.active_thoughts
    }

    pub fn store_temporary(&mut self, key: String, value: serde_json::Value) {
        self.temporary_data.insert(key, value);
    }

    pub fn retrieve_temporary(&self, key: &str) -> Option<&serde_json::Value> {
        self.temporary_data.get(key)
    }

    pub fn clear(&mut self) {
        self.current_focus = None;
        self.active_thoughts.clear();
        self.temporary_data.clear();
        self.attention_stack.clear();
    }
}

impl Default for WorkingMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// Episodic memory for storing execution experiences and outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemory {
    episodes: Vec<Episode>,
    max_episodes: usize,
}

impl EpisodicMemory {
    pub fn new() -> Self {
        Self {
            episodes: Vec::new(),
            max_episodes: 100,
        }
    }

    pub fn with_capacity(max_episodes: usize) -> Self {
        Self {
            episodes: Vec::new(),
            max_episodes,
        }
    }

    pub fn add_episode(&mut self, episode: Episode) {
        self.episodes.push(episode);
        // Keep memory bounded
        if self.episodes.len() > self.max_episodes {
            self.episodes.remove(0);
        }
    }

    pub fn get_recent_episodes(&self, count: usize) -> &[Episode] {
        let start = if self.episodes.len() > count {
            self.episodes.len() - count
        } else {
            0
        };
        &self.episodes[start..]
    }

    pub fn find_similar_episodes(&self, context_keywords: &[String]) -> Vec<&Episode> {
        self.episodes
            .iter()
            .filter(|episode| {
                context_keywords.iter().any(|keyword| {
                    episode
                        .context_summary
                        .to_lowercase()
                        .contains(&keyword.to_lowercase())
                        || episode
                            .action_taken
                            .to_lowercase()
                            .contains(&keyword.to_lowercase())
                })
            })
            .collect()
    }

    pub fn get_success_rate(&self) -> f64 {
        if self.episodes.is_empty() {
            return 0.0;
        }

        let successful = self.episodes.iter().filter(|ep| ep.success).count();
        successful as f64 / self.episodes.len() as f64
    }
}

impl Default for EpisodicMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual episode in episodic memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub context_summary: String,
    pub action_taken: String,
    pub outcome: String,
    pub success: bool,
    pub duration: std::time::Duration,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Episode {
    pub fn new(
        context_summary: String,
        action_taken: String,
        outcome: String,
        success: bool,
        duration: std::time::Duration,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            context_summary,
            action_taken,
            outcome,
            success,
            duration,
            metadata: HashMap::new(),
        }
    }
}

/// Semantic memory for storing domain knowledge and learned patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMemory {
    concepts: HashMap<String, Concept>,
    relations: Vec<Relation>,
    patterns: Vec<Pattern>,
}

impl SemanticMemory {
    pub fn new() -> Self {
        Self {
            concepts: HashMap::new(),
            relations: Vec::new(),
            patterns: Vec::new(),
        }
    }

    pub fn add_concept(&mut self, concept: Concept) {
        self.concepts.insert(concept.name.clone(), concept);
    }

    pub fn get_concept(&self, name: &str) -> Option<&Concept> {
        self.concepts.get(name)
    }

    pub fn add_relation(&mut self, relation: Relation) {
        self.relations.push(relation);
    }

    pub fn find_related_concepts(&self, concept_name: &str) -> Vec<&Concept> {
        let related_names: Vec<&String> = self
            .relations
            .iter()
            .filter_map(|rel| {
                if rel.from == concept_name {
                    Some(&rel.to)
                } else if rel.to == concept_name {
                    Some(&rel.from)
                } else {
                    None
                }
            })
            .collect();

        related_names
            .into_iter()
            .filter_map(|name| self.concepts.get(name))
            .collect()
    }

    pub fn add_pattern(&mut self, pattern: Pattern) {
        self.patterns.push(pattern);
    }

    pub fn find_matching_patterns(&self, keywords: &[String]) -> Vec<&Pattern> {
        self.patterns
            .iter()
            .filter(|pattern| {
                keywords.iter().any(|keyword| {
                    pattern
                        .triggers
                        .iter()
                        .any(|trigger| trigger.to_lowercase().contains(&keyword.to_lowercase()))
                })
            })
            .collect()
    }
}

impl Default for SemanticMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// Concept in semantic memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub name: String,
    pub description: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub confidence: f64,
    pub usage_count: u32,
}

impl Concept {
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            properties: HashMap::new(),
            confidence: 1.0,
            usage_count: 1,
        }
    }
}

/// Relation between concepts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub relation_type: String,
    pub strength: f64,
}

/// Learned pattern in semantic memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub name: String,
    pub triggers: Vec<String>,
    pub conditions: Vec<String>,
    pub actions: Vec<String>,
    pub success_rate: f64,
    pub usage_count: u32,
}

impl Pattern {
    pub fn new(name: String, triggers: Vec<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            triggers,
            conditions: Vec::new(),
            actions: Vec::new(),
            success_rate: 0.0,
            usage_count: 0,
        }
    }
}
