//! Advanced usage example demonstrating dptree integration and complex workflow patterns.

use std::time::Duration;

use pocketflow_rs::prelude::*;

// Define a complex workflow state for order processing
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum OrderState {
    Received,
    ValidatingPayment,
    PaymentApproved,
    PaymentRejected,
    ProcessingOrder,
    CheckingInventory,
    InsufficientStock,
    Packaging,
    Shipped,
    Delivered,
    Cancelled,
    Failed,
}

impl FlowState for OrderState {
    fn is_terminal(&self) -> bool {
        matches!(
            self,
            OrderState::Delivered
                | OrderState::Cancelled
                | OrderState::Failed
                | OrderState::PaymentRejected
                | OrderState::InsufficientStock
        )
    }

    fn can_transition_to(&self, target: &Self) -> bool {
        use OrderState::*;
        match (self, target) {
            (Received, ValidatingPayment) => true,
            (ValidatingPayment, PaymentApproved | PaymentRejected) => true,
            (PaymentApproved, ProcessingOrder) => true,
            (ProcessingOrder, CheckingInventory) => true,
            (CheckingInventory, Packaging | InsufficientStock) => true,
            (Packaging, Shipped) => true,
            (Shipped, Delivered) => true,
            (_, Cancelled | Failed) => true,
            _ => false,
        }
    }
}

// Payment validation node
#[derive(Debug)]
struct PaymentValidator;

#[async_trait]
impl Node for PaymentValidator {
    type State = OrderState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("🔍 Validating payment...");

        let order_total: f64 = context.get_json("order_total")?.unwrap_or(0.0);
        let payment_method: String = context.get_json("payment_method")?.unwrap_or_default();

        // Simulate payment validation
        tokio::time::sleep(Duration::from_millis(100)).await;

        if order_total > 0.0 && !payment_method.is_empty() {
            context.set("payment_validated_at", chrono::Utc::now())?;
            context.set("payment_status", "approved")?;
            println!("✅ Payment approved for ${:.2}", order_total);
            Ok((context, OrderState::PaymentApproved))
        } else {
            context.set("payment_status", "rejected")?;
            context.set("rejection_reason", "Invalid payment details")?;
            println!("❌ Payment rejected");
            Ok((context, OrderState::PaymentRejected))
        }
    }

    fn name(&self) -> String {
        "payment_validator".to_string()
    }
}

// Inventory checker node
#[derive(Debug)]
struct InventoryChecker;

#[async_trait]
impl Node for InventoryChecker {
    type State = OrderState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("📦 Checking inventory...");

        let items: Vec<String> = context.get_json("order_items")?.unwrap_or_default();

        // Simulate inventory check
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Simulate stock availability (80% chance of success)
        let has_stock = items.len() > 0 && fastrand::f32() > 0.2;

        if has_stock {
            context.set("inventory_checked_at", chrono::Utc::now())?;
            context.set("inventory_status", "available")?;
            println!("✅ All items in stock");
            Ok((context, OrderState::Packaging))
        } else {
            context.set("inventory_status", "insufficient")?;
            context.set("stock_issue", "Some items out of stock")?;
            println!("❌ Insufficient stock");
            Ok((context, OrderState::InsufficientStock))
        }
    }

    fn name(&self) -> String {
        "inventory_checker".to_string()
    }
}

// Packaging node
#[derive(Debug)]
struct PackagingProcessor;

#[async_trait]
impl Node for PackagingProcessor {
    type State = OrderState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("📋 Processing packaging...");

        let items: Vec<String> = context.get_json("order_items")?.unwrap_or_default();

        // Simulate packaging time
        tokio::time::sleep(Duration::from_millis(200)).await;

        context.set("packaged_at", chrono::Utc::now())?;
        context.set(
            "tracking_number",
            format!("TRK{}", fastrand::u32(100000..999999)),
        )?;
        context.set("package_weight", items.len() as f64 * 0.5)?;

        println!("✅ Order packaged with {} items", items.len());
        Ok((context, OrderState::Shipped))
    }

    fn name(&self) -> String {
        "packaging_processor".to_string()
    }
}

// Shipping processor
#[derive(Debug)]
struct ShippingProcessor;

#[async_trait]
impl Node for ShippingProcessor {
    type State = OrderState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("🚚 Processing shipment...");

        let tracking_number: String = context.get_json("tracking_number")?.unwrap_or_default();

        // Simulate shipping processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        context.set("shipped_at", chrono::Utc::now())?;
        context.set("delivery_estimate", "2-3 business days")?;

        println!("✅ Order shipped with tracking: {}", tracking_number);
        Ok((context, OrderState::Delivered))
    }

    fn name(&self) -> String {
        "shipping_processor".to_string()
    }
}

// Simple state transition nodes
#[derive(Debug)]
struct StateTransitionNode(OrderState);

#[async_trait]
impl Node for StateTransitionNode {
    type State = OrderState;

    async fn execute(&self, context: Context) -> Result<(Context, Self::State)> {
        Ok((context, self.0.clone()))
    }

    fn name(&self) -> String {
        format!("transition_to_{:?}", self.0)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Starting Advanced PocketFlow-RS Order Processing Example\n");

    // Create the advanced flow with dptree integration
    let flow = AdvancedFlow::builder()
        .name("order_processing_flow")
        .initial_state(OrderState::Received)
        // Add middleware for logging
        .middleware(|execution| {
            println!("📊 Step {}: {:?}", execution.steps, execution.current_state);
            execution
        })
        // Define state transitions and handlers
        .on_state(
            OrderState::Received,
            StateTransitionNode(OrderState::ValidatingPayment),
        )
        .on_state(OrderState::ValidatingPayment, PaymentValidator)
        .on_state(
            OrderState::PaymentApproved,
            StateTransitionNode(OrderState::ProcessingOrder),
        )
        .on_state(
            OrderState::ProcessingOrder,
            StateTransitionNode(OrderState::CheckingInventory),
        )
        .on_state(OrderState::CheckingInventory, InventoryChecker)
        .on_state(OrderState::Packaging, PackagingProcessor)
        .on_state(OrderState::Shipped, ShippingProcessor)
        // Add conditional handling for high-value orders
        .when(|execution| async move {
            if let Ok(Some(total)) = execution.context.get_json::<f64>("order_total") {
                total > 1000.0
            } else {
                false
            }
        })
        .then(StateTransitionNode(OrderState::Delivered)) // Skip some steps for high-value orders
        .build()?;

    // Test multiple order scenarios
    let test_cases = vec![
        (
            "Regular Order",
            serde_json::json!({
                "order_id": "ORD001",
                "order_total": 299.99,
                "payment_method": "credit_card",
                "order_items": ["laptop_stand", "wireless_mouse"]
            }),
        ),
        (
            "High-Value Order",
            serde_json::json!({
                "order_id": "ORD002",
                "order_total": 1599.99,
                "payment_method": "bank_transfer",
                "order_items": ["gaming_laptop", "mechanical_keyboard", "gaming_headset"]
            }),
        ),
        (
            "Invalid Payment Order",
            serde_json::json!({
                "order_id": "ORD003",
                "order_total": 0.0,
                "payment_method": "",
                "order_items": ["cheap_cable"]
            }),
        ),
    ];

    // Create a flow registry and register our flow
    let mut registry = FlowRegistry::new();
    registry.register("order_processing".to_string(), flow);

    // Process each test case
    for (test_name, order_data) in test_cases {
        println!("\n{}", "=".repeat(60));
        println!("🧪 Test Case: {}", test_name);
        println!("{}", "=".repeat(60));

        let mut context = Context::new();

        // Set order data from JSON
        for (key, value) in order_data.as_object().unwrap() {
            context.set_json(key, value)?;
        }

        let start_time = std::time::Instant::now();

        match registry.execute("order_processing", context).await {
            Ok(result) => {
                println!("\n✅ Flow completed successfully!");
                println!("📈 Final State: {:?}", result.final_state);
                println!("⏱️  Execution Time: {:?}", result.duration);
                println!("👣 Steps Executed: {}", result.steps);

                if let Ok(Some(tracking)) = result.context.get_json::<String>("tracking_number") {
                    println!("📦 Tracking Number: {}", tracking);
                }

                if let Ok(Some(status)) = result.context.get_json::<String>("payment_status") {
                    println!("💳 Payment Status: {}", status);
                }
            }
            Err(e) => {
                println!("\n❌ Flow execution failed: {}", e);
            }
        }

        println!("⏱️  Total Test Time: {:?}", start_time.elapsed());
    }

    println!("\n{}", "=".repeat(60));
    println!("🎉 Advanced Flow Example Completed");
    println!("=".repeat(60));

    // Demonstrate flow registry capabilities
    println!("\n📋 Available Flows:");
    for flow_name in registry.list_flows() {
        println!("  - {}", flow_name);
    }

    Ok(())
}
