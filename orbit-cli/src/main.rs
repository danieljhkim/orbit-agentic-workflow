use clap::{Args, Parser, Subcommand};
use orbit_core::OrbitRuntime;
use serde_json::{Map, Value};

#[derive(Parser)]
#[command(name = "orbit")]
#[command(about = "Orbit v2.1 CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Tool(ToolCommand),
    Task(TaskCommand),
    Audit(AuditCommand),
    Job(JobCommand),
    Watch(WatchCommand),
}

#[derive(Args)]
struct ToolCommand {
    #[command(subcommand)]
    command: ToolSubcommand,
}

#[derive(Subcommand)]
enum ToolSubcommand {
    Run(ToolRunArgs),
}

#[derive(Args)]
struct ToolRunArgs {
    name: String,
    #[arg(long)]
    path: Option<String>,
    #[arg(long)]
    content: Option<String>,
    #[arg(long)]
    program: Option<String>,
    #[arg(long = "arg")]
    args: Vec<String>,
    #[arg(long)]
    timeout_ms: Option<u64>,
}

#[derive(Args)]
struct TaskCommand {
    #[command(subcommand)]
    command: TaskSubcommand,
}

#[derive(Subcommand)]
enum TaskSubcommand {
    Add(TaskAddArgs),
    List,
}

#[derive(Args)]
struct TaskAddArgs {
    title: String,
}

#[derive(Args)]
struct AuditCommand {
    #[command(subcommand)]
    command: AuditSubcommand,
}

#[derive(Subcommand)]
enum AuditSubcommand {
    List(AuditListArgs),
}

#[derive(Args)]
struct AuditListArgs {
    #[arg(long, default_value_t = 20)]
    limit: usize,
}

#[derive(Args)]
struct JobCommand {
    #[command(subcommand)]
    command: JobSubcommand,
}

#[derive(Subcommand)]
enum JobSubcommand {
    Run,
}

#[derive(Args)]
struct WatchCommand {
    #[command(subcommand)]
    command: WatchSubcommand,
}

#[derive(Subcommand)]
enum WatchSubcommand {
    Run(WatchRunArgs),
}

#[derive(Args)]
struct WatchRunArgs {
    #[arg(long, default_value = ".")]
    path: String,
}

fn main() {
    let cli = Cli::parse();
    let data_root = OrbitRuntime::default_data_root();
    let runtime = match OrbitRuntime::from_data_root(&data_root) {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("failed to initialize runtime: {err}");
            std::process::exit(1);
        }
    };

    let outcome = match cli.command {
        Commands::Tool(cmd) => run_tool(&runtime, cmd),
        Commands::Task(cmd) => run_task(&runtime, cmd),
        Commands::Audit(cmd) => run_audit(&runtime, cmd),
        Commands::Job(cmd) => run_job(&runtime, cmd),
        Commands::Watch(cmd) => run_watch(&runtime, cmd),
    };

    if let Err(err) = outcome {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run_tool(runtime: &OrbitRuntime, cmd: ToolCommand) -> Result<(), orbit_types::OrbitError> {
    match cmd.command {
        ToolSubcommand::Run(args) => {
            let mut input = Map::new();
            if let Some(path) = args.path {
                input.insert("path".to_string(), Value::String(path));
            }
            if let Some(content) = args.content {
                input.insert("content".to_string(), Value::String(content));
            }
            if let Some(program) = args.program {
                input.insert("program".to_string(), Value::String(program));
            }
            if !args.args.is_empty() {
                input.insert(
                    "args".to_string(),
                    Value::Array(args.args.into_iter().map(Value::String).collect()),
                );
            }
            if let Some(timeout_ms) = args.timeout_ms {
                input.insert("timeout_ms".to_string(), Value::Number(timeout_ms.into()));
            }

            let output = runtime.run_tool(&args.name, Value::Object(input))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&output)
                    .map_err(|e| orbit_types::OrbitError::Execution(e.to_string()))?
            );
        }
    }

    Ok(())
}

fn run_task(runtime: &OrbitRuntime, cmd: TaskCommand) -> Result<(), orbit_types::OrbitError> {
    match cmd.command {
        TaskSubcommand::Add(args) => {
            let task = runtime.add_task(&args.title)?;
            println!("{}\t{}", task.id, task.title);
        }
        TaskSubcommand::List => {
            for task in runtime.list_tasks()? {
                println!(
                    "{}\t{}\t{}",
                    task.id,
                    task.created_at.to_rfc3339(),
                    task.title
                );
            }
        }
    }
    Ok(())
}

fn run_audit(runtime: &OrbitRuntime, cmd: AuditCommand) -> Result<(), orbit_types::OrbitError> {
    match cmd.command {
        AuditSubcommand::List(args) => {
            for audit in runtime.list_audits(args.limit)? {
                println!(
                    "{}\t{}\t{}\t{}",
                    audit.id,
                    audit.created_at.to_rfc3339(),
                    audit.event_type,
                    audit.message
                );
            }
        }
    }
    Ok(())
}

fn run_job(runtime: &OrbitRuntime, cmd: JobCommand) -> Result<(), orbit_types::OrbitError> {
    match cmd.command {
        JobSubcommand::Run => {
            let count = runtime.run_jobs()?;
            println!("ran_jobs={count}");
        }
    }
    Ok(())
}

fn run_watch(runtime: &OrbitRuntime, cmd: WatchCommand) -> Result<(), orbit_types::OrbitError> {
    match cmd.command {
        WatchSubcommand::Run(args) => {
            runtime.run_watch_once(&args.path)?;
            println!("watch trigger recorded for {}", args.path);
        }
    }
    Ok(())
}
