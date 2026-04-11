use clap::{Args, Subcommand};
use orbit_core::knowledge_stats::{KnowledgeStatsSummary, aggregate as aggregate_knowledge_stats};
use orbit_core::{
    ActivityInvocationMetrics, OrbitError, OrbitRuntime, TaskInvocationMetrics,
    ToolInvocationMetrics,
};
use serde_json::json;

use crate::command::Execute;

const LIMITATIONS_HELP: &str = "Known limitations:\n  - Subagent attribution folds into the parent invocation totals.\n  - cache_read_tokens are reported separately from input_tokens.\n  - Multi-task invocations are fully attributed to every tagged task.\n  - Non-Claude providers currently emit zero traces.";

#[derive(Args)]
#[command(
    about = "Inspect token, tool-call, and knowledge-pack metrics",
    after_help = LIMITATIONS_HELP
)]
pub struct MetricsCommand {
    /// Limit the number of most-recent job runs considered for knowledge metrics in the overview.
    #[arg(long)]
    pub limit: Option<usize>,
    /// Output overview as JSON
    #[arg(long)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Option<MetricsSubcommand>,
}

impl Execute for MetricsCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self.command {
            Some(command) => command.execute(runtime),
            None => MetricsOverviewArgs {
                limit: self.limit,
                json: self.json,
            }
            .execute(runtime),
        }
    }
}

#[derive(Subcommand)]
pub enum MetricsSubcommand {
    /// Show an overview of knowledge, activity, and tool metrics
    Overview(MetricsOverviewArgs),
    /// Show knowledge-pack usage and savings metrics
    Knowledge(MetricsKnowledgeArgs),
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
            MetricsSubcommand::Overview(args) => args.execute(runtime),
            MetricsSubcommand::Knowledge(args) => args.execute(runtime),
            MetricsSubcommand::Activity(args) => args.execute(runtime),
            MetricsSubcommand::Task(args) => args.execute(runtime),
            MetricsSubcommand::Tools(args) => args.execute(runtime),
        }
    }
}

#[derive(Args)]
pub struct MetricsOverviewArgs {
    /// Limit the number of most-recent job runs considered for knowledge metrics
    #[arg(long)]
    pub limit: Option<usize>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for MetricsOverviewArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let knowledge = load_knowledge_summary(runtime, self.limit)?;
        let activity = runtime.activity_invocation_metrics()?;
        let tools = runtime.tool_invocation_metrics()?;

        if self.json {
            return crate::output::json::print_pretty(&json!({
                "knowledge": knowledge,
                "activity": activity,
                "tools": tools,
            }));
        }

        print_knowledge_summary(&knowledge);
        println!();
        print_activity_rows(&activity);
        println!();
        print_tool_rows(&tools);
        Ok(())
    }
}

#[derive(Args)]
pub struct MetricsKnowledgeArgs {
    /// Limit the number of most-recent job runs considered
    #[arg(long)]
    pub limit: Option<usize>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for MetricsKnowledgeArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let summary = load_knowledge_summary(runtime, self.limit)?;

        if self.json {
            return crate::output::json::print_pretty(
                &serde_json::to_value(&summary).map_err(|e| OrbitError::Store(e.to_string()))?,
            );
        }

        print_knowledge_summary(&summary);
        Ok(())
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
        print_activity_rows(&rows);
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
        print_tool_rows(&rows);
        Ok(())
    }
}

fn load_knowledge_summary(
    runtime: &OrbitRuntime,
    limit: Option<usize>,
) -> Result<KnowledgeStatsSummary, OrbitError> {
    let runs = runtime.list_job_runs(orbit_core::command::job_run::JobRunListParams {
        limit,
        ..Default::default()
    })?;
    Ok(aggregate_knowledge_stats(&runs))
}

fn print_knowledge_summary(summary: &KnowledgeStatsSummary) {
    if summary.total_runs == 0 {
        println!("No knowledge usage metrics found.");
        return;
    }

    println!("orbit metrics knowledge");
    println!();
    println!(
        "Runs:        {} total | {} with pack | {} fallback ({:.1}% fallback rate)",
        summary.total_runs,
        summary.pack_runs,
        summary.fallback_runs,
        summary.fallback_rate * 100.0
    );
    println!();

    if let Some(compression) = &summary.compression {
        println!("Compression (tokenized, cl100k_base):");
        println!("  Mean:      {:.1}x", compression.mean);
        println!("  p50:       {:.1}x", compression.p50);
        println!("  p90:       {:.1}x", compression.p90);
        println!("  Min:       {:.1}x", compression.min);
        println!();
    }

    println!("Double-read guard:");
    println!(
        "  Mean rate: {:.2} ({:.0}% of baseline tokens re-read via fs.read)",
        summary.double_read.mean_rate,
        summary.double_read.mean_rate * 100.0
    );
    println!(
        "  Runs >50%: {} / {} (flag for investigation)",
        summary.double_read.runs_over_fifty_percent, summary.double_read.measured_runs
    );
    println!();

    println!("Total LLM input tokens per activity:");
    println!(
        "  With pack:    avg {:.0} tokens",
        summary.total_llm_input_tokens.with_pack_avg
    );
    println!(
        "  Without pack: avg {:.0} tokens",
        summary.total_llm_input_tokens.without_pack_avg
    );
    if let Some(savings) = summary.total_llm_input_tokens.estimated_savings {
        println!("  Estimated savings: {:.0}%", savings * 100.0);
    }
}

fn print_activity_rows(rows: &[ActivityInvocationMetrics]) {
    if rows.is_empty() {
        println!("No invocation metrics found.");
        return;
    }

    println!("Activity Metrics:");
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
            Cell::new(&row.activity_id),
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

fn print_tool_rows(rows: &[ToolInvocationMetrics]) {
    if rows.is_empty() {
        println!("No tool call metrics found.");
        return;
    }

    println!("Tool Metrics:");
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
            Cell::new(&row.activity_id),
            Cell::new(&row.tool_name),
            Cell::new(row.call_count),
            Cell::new(format_decimal(row.avg_result_bytes)),
            Cell::new(row.total_result_bytes),
        ]);
    }
    println!("{table}");
}

fn format_decimal(value: f64) -> String {
    format!("{value:.1}")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::Utc;
    use orbit_core::OrbitRuntime;
    use orbit_types::{JobRun, JobRunState, KnowledgeRunMetrics};
    use tempfile::tempdir;

    use super::load_knowledge_summary;

    fn seed_run(root: &std::path::Path, run_id: &str, metrics: KnowledgeRunMetrics) {
        let run_dir = root.join("jobs/runs/knowledge-job").join(run_id);
        fs::create_dir_all(&run_dir).expect("run dir");
        let run = JobRun {
            run_id: run_id.to_string(),
            job_id: "knowledge-job".to_string(),
            attempt: 1,
            state: JobRunState::Success,
            scheduled_at: Utc::now(),
            started_at: None,
            finished_at: None,
            duration_ms: None,
            created_at: Utc::now(),
            pid: None,
            pid_start_time: None,
            input: None,
            retry_source_run_id: None,
            knowledge_metrics: Some(metrics),
            steps: vec![],
        };
        let yaml = serde_yaml::to_string(&serde_json::json!({
            "schemaVersion": 1,
            "run": run,
        }))
        .expect("serialize");
        fs::write(run_dir.join("jrun.yaml"), yaml).expect("write run");
    }

    #[test]
    fn load_knowledge_summary_reads_recent_job_run_fixture() {
        let repo = tempdir().expect("tempdir");
        let orbit_root = repo.path().join(".orbit");
        fs::create_dir_all(&orbit_root).expect("orbit root");

        seed_run(
            &orbit_root,
            "r1",
            KnowledgeRunMetrics {
                raw_read_token_baseline: 100,
                knowledge_pack_tokens: Some(20),
                compression_ratio: Some(5.0),
                actual_fs_read_tokens_during_run: 10,
                double_read_rate: Some(0.1),
                knowledge_pack_used: true,
                knowledge_pack_unresolved_count: 0,
                total_llm_input_tokens: 400,
            },
        );
        seed_run(
            &orbit_root,
            "r2",
            KnowledgeRunMetrics {
                raw_read_token_baseline: 120,
                knowledge_pack_tokens: Some(30),
                compression_ratio: Some(4.0),
                actual_fs_read_tokens_during_run: 12,
                double_read_rate: Some(0.1),
                knowledge_pack_used: true,
                knowledge_pack_unresolved_count: 1,
                total_llm_input_tokens: 500,
            },
        );
        seed_run(
            &orbit_root,
            "r3",
            KnowledgeRunMetrics {
                raw_read_token_baseline: 90,
                knowledge_pack_tokens: Some(45),
                compression_ratio: Some(2.0),
                actual_fs_read_tokens_during_run: 63,
                double_read_rate: Some(0.7),
                knowledge_pack_used: true,
                knowledge_pack_unresolved_count: 2,
                total_llm_input_tokens: 700,
            },
        );
        seed_run(
            &orbit_root,
            "r4",
            KnowledgeRunMetrics {
                raw_read_token_baseline: 80,
                knowledge_pack_tokens: None,
                compression_ratio: None,
                actual_fs_read_tokens_during_run: 16,
                double_read_rate: Some(0.2),
                knowledge_pack_used: false,
                knowledge_pack_unresolved_count: 0,
                total_llm_input_tokens: 900,
            },
        );
        seed_run(
            &orbit_root,
            "r5",
            KnowledgeRunMetrics {
                raw_read_token_baseline: 70,
                knowledge_pack_tokens: None,
                compression_ratio: None,
                actual_fs_read_tokens_during_run: 21,
                double_read_rate: Some(0.3),
                knowledge_pack_used: false,
                knowledge_pack_unresolved_count: 0,
                total_llm_input_tokens: 1000,
            },
        );

        let runtime = OrbitRuntime::from_roots(&orbit_root, &orbit_root).expect("runtime");
        let summary = load_knowledge_summary(&runtime, None).expect("summary");

        assert_eq!(summary.total_runs, 5);
        assert_eq!(summary.pack_runs, 3);
        assert_eq!(summary.fallback_runs, 2);
        assert!(summary.compression.is_some());
        assert_eq!(summary.double_read.runs_over_fifty_percent, 1);
        assert_eq!(summary.double_read.measured_runs, 5);
        assert!(summary.total_llm_input_tokens.estimated_savings.is_some());
    }
}
