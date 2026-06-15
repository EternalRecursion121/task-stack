use serde::Serialize;
use std::process::Command;

/// Result of probing the aerospace CLI / server.
#[derive(Serialize, Clone, Debug)]
pub struct AeroStatus {
    pub installed: bool,
    pub server_enabled: bool,
    pub message: Option<String>,
}

const DISABLED_MARKER: &str = "server is disabled";

/// Run an `aerospace` subcommand. Returns Ok(stdout) on success, Err(message) otherwise.
/// A disabled server is reported as a distinct, friendly error.
fn run(args: &[&str]) -> Result<String, String> {
    let output = Command::new("aerospace").args(args).output();
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            if out.status.success() {
                Ok(stdout)
            } else if stderr.contains(DISABLED_MARKER) || stdout.contains(DISABLED_MARKER) {
                Err("AeroSpace server is disabled. Run `aerospace enable on`.".to_string())
            } else {
                Err(if stderr.trim().is_empty() {
                    "aerospace command failed".to_string()
                } else {
                    stderr.trim().to_string()
                })
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err("AeroSpace is not installed.".to_string())
            } else {
                Err(format!("failed to run aerospace: {e}"))
            }
        }
    }
}

pub fn status() -> AeroStatus {
    match run(&["list-workspaces", "--focused"]) {
        Ok(_) => AeroStatus {
            installed: true,
            server_enabled: true,
            message: None,
        },
        Err(msg) => {
            let installed = !msg.contains("not installed");
            let server_enabled = false;
            AeroStatus {
                installed,
                server_enabled,
                message: Some(msg),
            }
        }
    }
}

pub fn list_workspaces() -> Result<Vec<String>, String> {
    let out = run(&["list-workspaces", "--all"])?;
    Ok(out
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

pub fn focused_workspace() -> Result<String, String> {
    let out = run(&["list-workspaces", "--focused"])?;
    Ok(out.trim().to_string())
}

pub fn focus_workspace(name: &str, summon: bool) -> Result<(), String> {
    if summon {
        run(&["summon-workspace", name]).map(|_| ())
    } else {
        run(&["workspace", name]).map(|_| ())
    }
}

pub fn focus_window(window_id: &str) -> Result<(), String> {
    run(&["focus", "--window-id", window_id]).map(|_| ())
}

/// Capture the current multi-monitor arrangement: the visible workspace on every
/// monitor. AeroSpace workspaces are single-monitor, so a "scene" is the set of
/// workspaces visible across all screens at once. The focused workspace is moved
/// to the end so that replaying the scene leaves keyboard focus where it was.
pub fn visible_scene() -> Result<Vec<String>, String> {
    let out = run(&["list-workspaces", "--monitor", "all", "--visible"])?;
    let mut names: Vec<String> = out
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    // Dedupe (one workspace per monitor) while preserving order.
    let mut seen = std::collections::HashSet::new();
    names.retain(|n| seen.insert(n.clone()));

    // Replay in an order that ends on the previously-focused workspace.
    if let Ok(focused) = focused_workspace() {
        if let Some(pos) = names.iter().position(|n| n == &focused) {
            let f = names.remove(pos);
            names.push(f);
        }
    }
    Ok(names)
}

/// Restore a scene by focusing each workspace in turn. Each `workspace <name>`
/// only affects its own monitor, so the screens end up showing the captured
/// arrangement; the last name in the list receives keyboard focus. Best-effort
/// per workspace — a vanished workspace doesn't abort the rest.
pub fn focus_scene(names: &[String]) -> Result<(), String> {
    if names.is_empty() {
        return Err("empty scene".to_string());
    }
    let mut last = Ok(());
    for name in names {
        last = run(&["workspace", name]).map(|_| ());
    }
    last
}
