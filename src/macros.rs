// No imports needed at module level since macros are self-contained

// Helper to create closure-based nodes
#[macro_export]
macro_rules! create_node {
    ($exec:expr) => {{
        let f = $exec;
        std::sync::Arc::new($crate::node::sync::Node::new(f))
            as std::sync::Arc<dyn $crate::node::base::BaseNode>
    }};
}

#[macro_export]
macro_rules! create_async_node {
    ($exec:expr) => {{
        let f = $exec;
        std::sync::Arc::new($crate::node::async_node::AsyncNode::new(f))
            as std::sync::Arc<dyn $crate::node::base::AsyncBaseNode>
    }};
}

#[macro_export]
macro_rules! sync_node {
    ($f:expr) => {
        std::sync::Arc::new($crate::node::sync::Node::new($f))
    };
}

#[macro_export]
macro_rules! async_node {
    ($f:expr) => {
        std::sync::Arc::new($crate::node::async_node::AsyncNode::new($f))
    };
}
