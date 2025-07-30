//! State machine example using PocketFlow-RS with complex state transitions.

use std::time::Duration;

use pocketflow_core::prelude::*;

// Define a more complex state machine for an order processing system
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum OrderState {
    Received,
    PaymentPending,
    PaymentConfirmed,
    PaymentFailed,
    InventoryCheck,
    OutOfStock,
    InStock,
    Packaging,
    Shipped,
    Delivered,
    Cancelled,
    Refunded,
}

impl FlowState for OrderState {
    fn is_terminal(&self) -> bool {
        matches!(
            self,
            OrderState::Delivered | OrderState::Cancelled | OrderState::Refunded
        )
    }

    fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            // From terminal states, no transitions allowed
            (OrderState::Delivered | OrderState::Cancelled | OrderState::Refunded, _) => false,

            // From Received
            (OrderState::Received, OrderState::PaymentPending) => true,

            // From PaymentPending
            (
                OrderState::PaymentPending,
                OrderState::PaymentConfirmed | OrderState::PaymentFailed,
            ) => true,

            // From PaymentConfirmed
            (OrderState::PaymentConfirmed, OrderState::InventoryCheck) => true,

            // From PaymentFailed
            (OrderState::PaymentFailed, OrderState::Cancelled) => true,

            // From InventoryCheck
            (OrderState::InventoryCheck, OrderState::InStock | OrderState::OutOfStock) => true,

            // From OutOfStock
            (OrderState::OutOfStock, OrderState::Cancelled) => true,

            // From InStock
            (OrderState::InStock, OrderState::Packaging) => true,

            // From Packaging
            (OrderState::Packaging, OrderState::Shipped | OrderState::Cancelled) => true,

            // From Shipped
            (OrderState::Shipped, OrderState::Delivered) => true,

            // Allow cancellation from most states (except terminal)
            (
                OrderState::Received
                | OrderState::PaymentPending
                | OrderState::PaymentConfirmed
                | OrderState::InventoryCheck
                | OrderState::InStock
                | OrderState::Packaging,
                OrderState::Cancelled,
            ) => true,

            // From Cancelled to Refunded (if payment was made)
            (OrderState::Cancelled, OrderState::Refunded) => true,

            _ => false,
        }
    }
}

// Order data structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Order {
    id: String,
    amount: f64,
    items: Vec<String>,
    customer_id: String,
}

// Payment processing node
#[derive(Debug)]
struct PaymentProcessor;

#[async_trait]
impl Node for PaymentProcessor {
    type State = OrderState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("💳 Processing payment...");

        let order: Order = context
            .get_json("order")?
            .ok_or_else(|| FlowError::context("Order not found in context"))?;

        // Simulate payment processing delay
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Simulate payment success/failure based on amount
        let success = order.amount > 0.0 && order.amount < 1000.0; // Fail for very high amounts

        if success {
            context.set("payment_id", format!("pay_{}", order.id))?;
            context.set("payment_status", "confirmed")?;
            println!("✅ Payment confirmed for ${:.2}", order.amount);
            Ok((context, OrderState::PaymentConfirmed))
        } else {
            context.set("payment_status", "failed")?;
            context.set("error", "Payment processing failed")?;
            println!("❌ Payment failed for ${:.2}", order.amount);
            Ok((context, OrderState::PaymentFailed))
        }
    }

    fn name(&self) -> String {
        "PaymentProcessor".to_string()
    }
}

// Inventory check node
#[derive(Debug)]
struct InventoryChecker;

#[async_trait]
impl Node for InventoryChecker {
    type State = OrderState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("📦 Checking inventory...");

        let order: Order = context
            .get_json("order")?
            .ok_or_else(|| FlowError::context("Order not found in context"))?;

        // Simulate inventory check
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Simple inventory logic: items with "rare" in name are out of stock
        let out_of_stock_items: Vec<&String> = order
            .items
            .iter()
            .filter(|item| item.to_lowercase().contains("rare"))
            .collect();

        if out_of_stock_items.is_empty() {
            context.set("inventory_status", "available")?;
            println!("✅ All items in stock");
            Ok((context, OrderState::InStock))
        } else {
            context.set("inventory_status", "unavailable")?;
            context.set("out_of_stock_items", &out_of_stock_items)?;
            println!("❌ Some items out of stock: {:?}", out_of_stock_items);
            Ok((context, OrderState::OutOfStock))
        }
    }

    fn name(&self) -> String {
        "InventoryChecker".to_string()
    }
}

// Packaging node
#[derive(Debug)]
struct PackagingNode;

#[async_trait]
impl Node for PackagingNode {
    type State = OrderState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("📦 Packaging order...");

        let order: Order = context
            .get_json("order")?
            .ok_or_else(|| FlowError::context("Order not found in context"))?;

        // Simulate packaging time
        tokio::time::sleep(Duration::from_millis(400)).await;

        // Generate tracking number
        let tracking_number = format!("TRACK{}", order.id.to_uppercase());
        context.set("tracking_number", tracking_number.clone())?;
        context.set("package_status", "ready_to_ship")?;

        println!("✅ Order packaged, tracking: {}", tracking_number);
        Ok((context, OrderState::Shipped))
    }

    fn name(&self) -> String {
        "PackagingNode".to_string()
    }
}

// Cancellation node
#[derive(Debug)]
struct CancellationNode;

#[async_trait]
impl Node for CancellationNode {
    type State = OrderState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("🚫 Processing cancellation...");

        // Check if payment was made
        let payment_status: Option<String> = context.get_json("payment_status")?;
        let needs_refund = payment_status.as_deref() == Some("confirmed");

        context.set("cancellation_reason", "Customer requested")?;
        context.set("cancelled_at", chrono::Utc::now().to_rfc3339())?;

        if needs_refund {
            println!("💰 Refund needed");
            Ok((context, OrderState::Refunded))
        } else {
            println!("✅ Order cancelled (no refund needed)");
            Ok((context, OrderState::Cancelled))
        }
    }

    fn name(&self) -> String {
        "CancellationNode".to_string()
    }
}

// Helper function to create an order processing flow
fn create_order_flow() -> Result<SimpleFlow<OrderState>> {
    let flow = SimpleFlow::builder()
        .name("order_processing_flow")
        .initial_state(OrderState::Received)
        // Start with payment processing
        .node(
            OrderState::Received,
            pocketflow_core::node::helpers::passthrough(
                "OrderReceived",
                OrderState::PaymentPending,
            ),
        )
        // Payment processing
        .node(OrderState::PaymentPending, PaymentProcessor)
        // Inventory check after successful payment
        .node(
            OrderState::PaymentConfirmed,
            pocketflow_core::node::helpers::passthrough(
                "PaymentConfirmed",
                OrderState::InventoryCheck,
            ),
        )
        // Inventory checking
        .node(OrderState::InventoryCheck, InventoryChecker)
        // Packaging after inventory confirmation
        .node(
            OrderState::InStock,
            pocketflow_core::node::helpers::passthrough("InStock", OrderState::Packaging),
        )
        // Packaging process
        .node(OrderState::Packaging, PackagingNode)
        // Final delivery step
        .node(
            OrderState::Shipped,
            pocketflow_core::node::helpers::passthrough("Shipped", OrderState::Delivered),
        )
        // Handle failures
        .node(OrderState::PaymentFailed, CancellationNode)
        .node(OrderState::OutOfStock, CancellationNode)
        .build()?;

    Ok(flow)
}

async fn process_order(order: Order, should_cancel: bool) -> Result<()> {
    println!("\n🏪 Processing Order: {}", order.id);
    println!("Items: {:?}", order.items);
    println!("Amount: ${:.2}", order.amount);

    let flow = create_order_flow()?;

    // Create context with order data
    let mut context = Context::new();
    context.set("order", &order)?;
    context.set_metadata("processing_started", chrono::Utc::now().to_rfc3339())?;

    // Simulate cancellation for demonstration
    if should_cancel {
        context.set("cancellation_requested", true)?;
    }

    // Execute the flow
    let result = flow.execute(context).await?;

    // Display results
    println!("\n📊 Order Processing Results:");
    println!("Final State: {:?}", result.final_state);
    println!("Duration: {:?}", result.duration);
    println!("Steps: {}", result.steps);
    println!("Success: {}", result.success);

    // Show relevant context data based on final state
    match result.final_state {
        OrderState::Delivered => {
            if let Some(tracking) = result.context.get_json::<String>("tracking_number")? {
                println!("📦 Tracking Number: {}", tracking);
            }
        }
        OrderState::Cancelled | OrderState::Refunded => {
            if let Some(reason) = result.context.get_json::<String>("cancellation_reason")? {
                println!("🚫 Cancellation Reason: {}", reason);
            }
        }
        _ => {
            if let Some(error) = result.context.get_json::<String>("error")? {
                println!("❌ Error: {}", error);
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🏪 PocketFlow-RS State Machine Example: Order Processing System");

    // Test successful order
    let order1 = Order {
        id: "ORD001".to_string(),
        amount: 49.99,
        items: vec!["T-Shirt".to_string(), "Jeans".to_string()],
        customer_id: "CUST001".to_string(),
    };

    process_order(order1, false).await?;

    // Test order with payment failure (high amount)
    let order2 = Order {
        id: "ORD002".to_string(),
        amount: 1500.0, // This will trigger payment failure
        items: vec!["Expensive Watch".to_string()],
        customer_id: "CUST002".to_string(),
    };

    process_order(order2, false).await?;

    // Test order with out of stock items
    let order3 = Order {
        id: "ORD003".to_string(),
        amount: 299.99,
        items: vec!["Rare Collectible".to_string(), "Regular Item".to_string()],
        customer_id: "CUST003".to_string(),
    };

    process_order(order3, false).await?;

    // Test successful order with multiple items
    let order4 = Order {
        id: "ORD004".to_string(),
        amount: 129.99,
        items: vec![
            "Book".to_string(),
            "Notebook".to_string(),
            "Pen".to_string(),
        ],
        customer_id: "CUST004".to_string(),
    };

    process_order(order4, false).await?;

    println!("\n🎉 State Machine Example completed!");
    println!("\n📋 State Transition Summary:");
    println!(
        "✅ Successful flow: Received → PaymentPending → PaymentConfirmed → InventoryCheck → InStock → Packaging → Shipped → Delivered"
    );
    println!("💳 Payment failure: Received → PaymentPending → PaymentFailed → Cancelled");
    println!(
        "📦 Out of stock: Received → PaymentPending → PaymentConfirmed → InventoryCheck → OutOfStock → Cancelled"
    );

    Ok(())
}
