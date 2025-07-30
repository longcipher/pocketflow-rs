//! Batch processing example using PocketFlow-RS.

use std::time::Duration;

use pocketflow_core::prelude::*;

// Batch processing states
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum BatchState {
    Start,
    LoadingData,
    Processing,
    Validation,
    Saving,
    Complete,
    Error,
}

impl FlowState for BatchState {
    fn is_terminal(&self) -> bool {
        matches!(self, BatchState::Complete | BatchState::Error)
    }
}

// Data item for batch processing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DataItem {
    id: u32,
    value: String,
    priority: u8,
}

// Batch statistics
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct BatchStats {
    total_items: usize,
    processed_items: usize,
    failed_items: usize,
    processing_time_ms: u64,
}

// Data loader node - simulates loading data from a source
#[derive(Debug)]
struct DataLoaderNode {
    batch_size: usize,
}

impl DataLoaderNode {
    fn new(batch_size: usize) -> Self {
        Self { batch_size }
    }
}

#[async_trait]
impl Node for DataLoaderNode {
    type State = BatchState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("📥 Loading batch data (size: {})...", self.batch_size);

        // Simulate data loading delay
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Generate sample data
        let mut items = Vec::new();
        for i in 1..=self.batch_size {
            let item = DataItem {
                id: i as u32,
                value: format!("data_item_{}", i),
                priority: (i % 3 + 1) as u8, // Priority 1-3
            };
            items.push(item);
        }

        context.set("batch_data", &items)?;
        context.set("batch_size", self.batch_size)?;

        // Initialize statistics
        let stats = BatchStats {
            total_items: items.len(),
            processed_items: 0,
            failed_items: 0,
            processing_time_ms: 0,
        };
        context.set("batch_stats", &stats)?;

        println!("✅ Loaded {} items", items.len());
        Ok((context, BatchState::Processing))
    }

    fn name(&self) -> String {
        format!("DataLoader({})", self.batch_size)
    }
}

// Batch processor node - processes items in parallel
#[derive(Debug)]
struct BatchProcessorNode {
    chunk_size: usize,
}

impl BatchProcessorNode {
    fn new(chunk_size: usize) -> Self {
        Self { chunk_size }
    }

    // Simulate processing a single item
    async fn process_item(&self, item: &DataItem) -> Result<DataItem> {
        // Simulate processing time based on priority
        let delay_ms = match item.priority {
            1 => 50,  // Low priority - quick processing
            2 => 100, // Medium priority
            3 => 200, // High priority - more processing
            _ => 100,
        };

        tokio::time::sleep(Duration::from_millis(delay_ms)).await;

        // Simulate occasional failures (5% chance)
        if item.id % 20 == 0 {
            return Err(FlowError::context(format!(
                "Processing failed for item {}",
                item.id
            )));
        }

        // Transform the item
        let mut processed = item.clone();
        processed.value = format!("processed_{}", item.value);

        Ok(processed)
    }
}

#[async_trait]
impl Node for BatchProcessorNode {
    type State = BatchState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("⚙️ Processing batch data...");

        let start_time = std::time::Instant::now();

        let items: Vec<DataItem> = context.get_json("batch_data")?.unwrap_or_default();

        if items.is_empty() {
            return Ok((context, BatchState::Error));
        }

        let mut processed_items = Vec::new();
        let mut failed_count = 0;

        // Process items in chunks for better memory management
        for chunk in items.chunks(self.chunk_size) {
            println!("  📦 Processing chunk of {} items...", chunk.len());

            // Process chunk items in parallel
            let mut chunk_tasks = Vec::new();
            for item in chunk {
                let processor = self.clone();
                let item = item.clone();
                chunk_tasks.push(tokio::spawn(
                    async move { processor.process_item(&item).await },
                ));
            }

            // Collect results
            for task in chunk_tasks {
                match task.await {
                    Ok(Ok(processed_item)) => {
                        processed_items.push(processed_item);
                    }
                    Ok(Err(_)) | Err(_) => {
                        failed_count += 1;
                    }
                }
            }
        }

        let processing_time = start_time.elapsed();

        // Update statistics
        let mut stats: BatchStats = context.get_json("batch_stats")?.unwrap_or_default();
        stats.processed_items = processed_items.len();
        stats.failed_items = failed_count;
        stats.processing_time_ms = processing_time.as_millis() as u64;

        context.set("processed_data", &processed_items)?;
        context.set("batch_stats", &stats)?;

        println!("✅ Batch processing complete:");
        println!("  📊 Processed: {}", stats.processed_items);
        println!("  ❌ Failed: {}", stats.failed_items);
        println!("  ⏱️ Time: {}ms", stats.processing_time_ms);

        // Decide next state based on results
        if stats.failed_items == 0 {
            Ok((context, BatchState::Validation))
        } else if stats.processed_items > 0 {
            // Some items processed successfully, continue but with warnings
            context.set("has_warnings", true)?;
            Ok((context, BatchState::Validation))
        } else {
            // All items failed
            context.set("error", "All items failed processing")?;
            Ok((context, BatchState::Error))
        }
    }

    fn name(&self) -> String {
        format!("BatchProcessor(chunk:{})", self.chunk_size)
    }
}

impl Clone for BatchProcessorNode {
    fn clone(&self) -> Self {
        Self {
            chunk_size: self.chunk_size,
        }
    }
}

// Validation node - validates processed data
#[derive(Debug)]
struct ValidationNode {
    min_success_rate: f64,
}

impl ValidationNode {
    fn new(min_success_rate: f64) -> Self {
        Self { min_success_rate }
    }
}

#[async_trait]
impl Node for ValidationNode {
    type State = BatchState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("🔍 Validating processed data...");

        let stats: BatchStats = context.get_json("batch_stats")?.unwrap_or_default();

        // Calculate success rate
        let success_rate = if stats.total_items > 0 {
            stats.processed_items as f64 / stats.total_items as f64
        } else {
            0.0
        };

        context.set("success_rate", success_rate)?;

        println!("  📈 Success rate: {:.1}%", success_rate * 100.0);
        println!(
            "  📋 Minimum required: {:.1}%",
            self.min_success_rate * 100.0
        );

        if success_rate >= self.min_success_rate {
            context.set("validation_result", "passed")?;
            println!("✅ Validation passed");
            Ok((context, BatchState::Saving))
        } else {
            context.set("validation_result", "failed")?;
            context.set(
                "error",
                format!(
                    "Success rate {:.1}% below minimum {:.1}%",
                    success_rate * 100.0,
                    self.min_success_rate * 100.0
                ),
            )?;
            println!("❌ Validation failed");
            Ok((context, BatchState::Error))
        }
    }

    fn name(&self) -> String {
        format!("Validation(min:{:.0}%)", self.min_success_rate * 100.0)
    }
}

// Data saver node - simulates saving results
#[derive(Debug)]
struct DataSaverNode;

#[async_trait]
impl Node for DataSaverNode {
    type State = BatchState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("💾 Saving processed data...");

        let processed_data: Vec<DataItem> = context.get_json("processed_data")?.unwrap_or_default();

        // Simulate saving delay
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Generate output metadata
        let output_id = format!("batch_output_{}", chrono::Utc::now().timestamp());
        context.set("output_id", &output_id)?;
        context.set("saved_items_count", processed_data.len())?;

        println!(
            "✅ Saved {} items with ID: {}",
            processed_data.len(),
            output_id
        );
        Ok((context, BatchState::Complete))
    }

    fn name(&self) -> String {
        "DataSaver".to_string()
    }
}

// Create a batch processing flow
fn create_batch_flow(
    batch_size: usize,
    chunk_size: usize,
) -> Result<pocketflow_core::flow::SimpleFlow<BatchState>> {
    let flow = pocketflow_core::flow::SimpleFlow::builder()
        .name("batch_processing_flow")
        .initial_state(BatchState::Start)
        // Data loading
        .node(
            BatchState::Start,
            pocketflow_core::node::helpers::passthrough("BatchStart", BatchState::LoadingData),
        )
        .node(BatchState::LoadingData, DataLoaderNode::new(batch_size))
        // Batch processing
        .node(BatchState::Processing, BatchProcessorNode::new(chunk_size))
        // Validation
        .node(BatchState::Validation, ValidationNode::new(0.8))
        // Saving results
        .node(BatchState::Saving, DataSaverNode)
        .build()?;

    Ok(flow)
}

async fn run_batch_job(name: &str, batch_size: usize, chunk_size: usize) -> Result<()> {
    println!("\n🚀 Starting Batch Job: {}", name);
    println!("📋 Batch Size: {}, Chunk Size: {}", batch_size, chunk_size);

    let flow = create_batch_flow(batch_size, chunk_size)?;

    // Create initial context
    let mut context = Context::new();
    context.set_metadata("job_name", name)?;
    context.set_metadata("started_at", chrono::Utc::now().to_rfc3339())?;

    // Execute the batch job
    let result = flow.execute(context).await?;

    // Display results
    println!("\n📊 Batch Job Results for '{}':", name);
    println!("Final State: {:?}", result.final_state);
    println!("Total Duration: {:?}", result.duration);
    println!("Steps: {}", result.steps);
    println!("Success: {}", result.success);

    // Show detailed statistics
    if let Some(stats) = result.context.get_json::<BatchStats>("batch_stats")? {
        println!("\n📈 Processing Statistics:");
        println!("  Total Items: {}", stats.total_items);
        println!("  Processed: {}", stats.processed_items);
        println!("  Failed: {}", stats.failed_items);
        println!("  Processing Time: {}ms", stats.processing_time_ms);

        if let Some(success_rate) = result.context.get_json::<f64>("success_rate")? {
            println!("  Success Rate: {:.1}%", success_rate * 100.0);
        }
    }

    // Show output information if successful
    if result.final_state == BatchState::Complete {
        if let Some(output_id) = result.context.get_json::<String>("output_id")? {
            println!("💾 Output ID: {}", output_id);
        }
        if let Some(saved_count) = result.context.get_json::<usize>("saved_items_count")? {
            println!("💾 Saved Items: {}", saved_count);
        }
    }

    // Show error if failed
    if result.final_state == BatchState::Error {
        if let Some(error) = result.context.get_json::<String>("error")? {
            println!("❌ Error: {}", error);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("📦 PocketFlow-RS Batch Processing Example");

    // Run different batch processing scenarios

    // Small batch - should complete successfully
    run_batch_job("Small Batch", 10, 3).await?;

    // Medium batch - should complete successfully
    run_batch_job("Medium Batch", 50, 10).await?;

    // Large batch with small chunks - might have some failures but should pass validation
    run_batch_job("Large Batch", 100, 5).await?;

    // Very large batch - might fail validation due to simulated failures
    run_batch_job("Extra Large Batch", 200, 20).await?;

    println!("\n🎉 Batch Processing Examples completed!");

    println!("\n📋 Key Features Demonstrated:");
    println!("✅ Parallel processing within chunks");
    println!("✅ Configurable batch and chunk sizes");
    println!("✅ Error handling and partial success scenarios");
    println!("✅ Statistics tracking and validation");
    println!("✅ Async processing with proper resource management");

    Ok(())
}
