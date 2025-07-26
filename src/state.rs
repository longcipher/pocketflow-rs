//! State management for PocketFlow workflows.

/// Trait representing a state in the workflow.
///
/// States define the current position in the workflow and control
/// which transitions are valid.
pub trait FlowState:
    Clone + PartialEq + Eq + std::hash::Hash + std::fmt::Debug + Send + Sync + 'static
{
    /// Returns true if this is a terminal state (workflow should stop).
    fn is_terminal(&self) -> bool;

    /// Returns true if this state can transition to the target state.
    /// Default implementation allows all transitions.
    fn can_transition_to(&self, _target: &Self) -> bool {
        true
    }
}

/// A simple enum-based state for basic workflows.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SimpleState {
    /// Initial state
    Start,
    /// Processing state
    Processing,
    /// Successful completion
    Success,
    /// Error state
    Error,
    /// Custom state with a name
    Custom(String),
}

impl FlowState for SimpleState {
    fn is_terminal(&self) -> bool {
        matches!(self, SimpleState::Success | SimpleState::Error)
    }

    fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            // Can't transition from terminal states
            (SimpleState::Success | SimpleState::Error, _) => false,
            // Can transition from Start to Processing or terminal states
            (SimpleState::Start, _) => true,
            // Can transition from Processing to terminal states
            (SimpleState::Processing, SimpleState::Success | SimpleState::Error) => true,
            // Custom states can transition to any non-terminal state
            (SimpleState::Custom(_), target) => !target.is_terminal(),
            _ => false,
        }
    }
}

/// State transition information.
#[derive(Clone, Debug)]
pub struct StateTransition<S: FlowState> {
    /// Source state
    pub from: S,
    /// Target state
    pub to: S,
    /// Optional metadata about the transition
    pub metadata: Option<serde_json::Value>,
}

impl<S: FlowState> StateTransition<S> {
    /// Create a new state transition.
    pub fn new(from: S, to: S) -> Self {
        Self {
            from,
            to,
            metadata: None,
        }
    }

    /// Create a new state transition with metadata.
    pub fn with_metadata(from: S, to: S, metadata: serde_json::Value) -> Self {
        Self {
            from,
            to,
            metadata: Some(metadata),
        }
    }

    /// Check if this transition is valid.
    pub fn is_valid(&self) -> bool {
        self.from.can_transition_to(&self.to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_state_transitions() {
        let start = SimpleState::Start;
        let processing = SimpleState::Processing;
        let success = SimpleState::Success;
        let error = SimpleState::Error;

        // Valid transitions
        assert!(start.can_transition_to(&processing));
        assert!(start.can_transition_to(&success));
        assert!(start.can_transition_to(&error));
        assert!(processing.can_transition_to(&success));
        assert!(processing.can_transition_to(&error));

        // Invalid transitions
        assert!(!success.can_transition_to(&processing));
        assert!(!error.can_transition_to(&processing));
        assert!(!processing.can_transition_to(&start));
    }

    #[test]
    fn test_state_transition_validity() {
        let transition = StateTransition::new(SimpleState::Start, SimpleState::Processing);
        assert!(transition.is_valid());

        let invalid_transition =
            StateTransition::new(SimpleState::Success, SimpleState::Processing);
        assert!(!invalid_transition.is_valid());
    }

    #[test]
    fn test_terminal_states() {
        assert!(!SimpleState::Start.is_terminal());
        assert!(!SimpleState::Processing.is_terminal());
        assert!(SimpleState::Success.is_terminal());
        assert!(SimpleState::Error.is_terminal());
        assert!(!SimpleState::Custom("custom".to_string()).is_terminal());
    }
}
