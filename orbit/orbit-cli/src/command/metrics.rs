use clap::{Args, Subcommand};
use orbit_core::{OrbitError, OrbitRuntime, TaskInvocationMetrics};

use crate::command::Execute;

const LIMITATIONS_HELP: &str = "Known limitations:\n  - Subagent attribution folds into the parent invocation totals.\n  - cache_read_tokens are reported separately from input_tokens.\n  - Multi-task invocations are fully attributed to every tagged task.\n  - Non-Claude providers currently emit zero traces.";

#[derive(Args)]
#[command(about = "Inspect invocation token and tool-call metrics", after_help = LIMITATIONS_HELP)]
pub struct MetricsCommand {
    #[command(subcommand)]
    pub command: MetricsSubcommand,
}

impl Execute for MetricsCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        self.command.execute(runtime)
    }
}

#[derive(Subcommand)]
pub enum MetricsSubcommand {
    /// Show token rollups grouped by activity
    Activity(MetricsActivityArgs),
    /// Show token rollups for a task
    Task(MetricsTaskArgs),
    /// Show tool call frequency and result sizes grouped by activity
    Tools(MetricsToolsArgs),
}

impl Execute for MetricsSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            MetricsSubcommand::Activity(args) => args.execute(runtime),
            MetricsSubcommand::Task(args) => args.execute(runtime),
            MetricsSubcommand::Tools(args) => args.execute(runtime),
        }
    }
}

#[derive(Args)]
pub struct MetricsActivityArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for MetricsActivityArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let rows = runtime.activity_invocation_metrics()?;
        if self.json {
            return crate::output::json::print_pretty(
                &serde_json::to_value(&rows).map_err(|e| OrbitError::Store(e.to_string()))?,
            );
        }
        if rows.is_empty() {
            println!("No invocation metrics found.");
            return Ok(());
        }

        use comfy_table::Cell;
        let mut table = crate::output::table::build_table(&[
            "ACTIVITY",
            "INVOCATIONS",
            "AVG",
            "P50",
            "P95",
            "TOTAL",
            "INPUT",
            "CACHE READ",
            "CACHE CREATE",
            "OUTPUT",
            "TOOLS",
        ]);
        for row in rows {
            table.add_row(vec![
                Cell::new(row.activity_id),
                Cell::new(row.invocation_count),
                Cell::new(format_decimal(row.avg_tokens)),
                Cell::new(row.p50_tokens),
                Cell::new(row.p95_tokens),
                Cell::new(row.total_tokens),
                Cell::new(row.total_input_tokens),
                Cell::new(row.total_cache_read_tokens),
                Cell::new(row.total_cache_create_tokens),
                Cell::new(row.total_output_tokens),
                Cell::new(row.total_tool_calls),
            ]);
        }
        println!("{table}");
        Ok(())
    }
}

#[derive(Args)]
pub struct MetricsTaskArgs {
    /// Task ID
    pub id: String,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for MetricsTaskArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let row = runtime.task_invocation_metrics(&self.id)?;
        if self.json {
            return crate::output::json::print_pretty(
                &serde_json::to_value(&row).map_err(|e| OrbitError::Store(e.to_string()))?,
            );
        }
        print_task_metrics(&row);
        Ok(())
    }
}

#[derive(Args)]
pub struct MetricsToolsArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for MetricsToolsArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let rows = runtime.tool_invocation_metrics()?;
        if self.json {
            return crate::output::json::print_pretty(
                &serde_json::to_value(&rows).map_err(|e| OrbitError::Store(e.to_string()))?,
            );
        }
        if rows.is_empty() {
            println!("No tool call metrics found.");
            return Ok(());
        }

        use comfy_table::Cell;
        let mut table = crate::output::table::build_table(&[
            "ACTIVITY",
            "TOOL",
            "CALLS",
            "AVG RESULT BYTES",
            "TOTAL RESULT BYTES",
        ]);
        for row in rows {
            table.add_row(vec![
                Cell::new(row.activity_id),
                Cell::new(row.tool_name),
                Cell::new(row.call_count),
                Cell::new(format_decimal(row.avg_result_bytes)),
                Cell::new(row.total_result_bytes),
            ]);
        }
        println!("{table}");
        Ok(())
    }
}

fn print_task_metrics(row: &TaskInvocationMetrics) {
    use comfy_table::Cell;
    let mut table = crate::output::table::build_table(&[
        "TASK",
        "INVOCATIONS",
        "TOTAL",
        "INPUT",
        "CACHE READ",
        "CACHE CREATE",
        "OUTPUT",
        "TOOLS",
    ]);
    table.add_row(vec![
        Cell::new(&row.task_id),
        Cell::new(row.invocation_count),
        Cell::new(row.total_tokens),
        Cell::new(row.total_input_tokens),
        Cell::new(row.total_cache_read_tokens),
        Cell::new(row.total_cache_create_tokens),
        Cell::new(row.total_output_tokens),
        Cell::new(row.total_tool_calls),
    ]);
    println!("{table}");
}

fn format_decimal(value: f64) -> String {
    format!("{value:.1}")
}
