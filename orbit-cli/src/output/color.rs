use colored::Colorize;

pub fn status_color(status: &str) -> String {
    match status {
        "proposed" => status.yellow().to_string(),
        "backlog" => status.to_string(),
        "in-progress" => status.cyan().to_string(),
        "review" => status.magenta().to_string(),
        "done" => status.green().bold().to_string(),
        "rejected" | "blocked" => status.red().to_string(),
        "archived" => status.dimmed().to_string(),
        _ => status.to_string(),
    }
}

pub fn priority_color(priority: &str) -> String {
    match priority {
        "high" => priority.red().to_string(),
        "medium" => priority.yellow().to_string(),
        "low" => priority.dimmed().to_string(),
        _ => priority.to_string(),
    }
}

pub fn job_state_color(state: &str) -> String {
    match state {
        "success" | "active" => state.green().to_string(),
        "failed" | "error" | "disabled" => state.red().to_string(),
        "running" => state.cyan().to_string(),
        "pending" => state.yellow().to_string(),
        _ => state.to_string(),
    }
}

pub fn doctor_status_color(status: &str) -> String {
    match status {
        "ok" => status.green().to_string(),
        "warning" => status.yellow().to_string(),
        "ERROR" | "error" => status.red().bold().to_string(),
        _ => status.to_string(),
    }
}

pub fn bold(text: &str) -> String {
    text.bold().to_string()
}

pub fn dimmed(text: &str) -> String {
    text.dimmed().to_string()
}
