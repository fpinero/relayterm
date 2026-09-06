use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    process::Command,
};

fn allowed_dependencies(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "relayterm-domain" => Some(&[]),
        "relayterm-application" => Some(&["relayterm-domain"]),
        "relayterm-protocol" => Some(&[]),
        "relayterm-daemon" => Some(&[
            "relayterm-application",
            "relayterm-domain",
            "relayterm-protocol",
        ]),
        "relayterm-tui" => Some(&["relayterm-protocol"]),
        "relayterm-cli" => Some(&["relayterm-daemon", "relayterm-tui"]),
        _ => None,
    }
}

fn validate(metadata: &Value) -> Result<(), String> {
    let packages = metadata["packages"].as_array().ok_or("Missing packages")?;
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("Missing members")?;
    let by_id: HashMap<_, _> = packages
        .iter()
        .map(|p| (p["id"].as_str().unwrap(), p))
        .collect();
    for id in members {
        let package = by_id[id.as_str().unwrap()];
        let name = package["name"].as_str().unwrap();
        let allowed = allowed_dependencies(name).ok_or(format!("Unreviewed package {name}"))?;
        for dep in package["dependencies"].as_array().unwrap() {
            let dep_name = dep["name"].as_str().unwrap();
            // Inspect declarations, including renamed, optional and target-specific edges.
            if dep_name.starts_with("relayterm-") && !allowed.contains(&dep_name) {
                return Err(format!("Forbidden dependency {name} -> {dep_name}"));
            }
        }
    }
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or("Missing resolved graph")?;
    let edges: HashMap<_, _> = nodes
        .iter()
        .map(|n| (n["id"].as_str().unwrap(), n))
        .collect();
    for root in [
        "relayterm-domain",
        "relayterm-application",
        "relayterm-protocol",
    ] {
        let package = packages
            .iter()
            .find(|p| p["name"] == root)
            .ok_or("Missing core package")?;
        let mut pending = vec![package["id"].as_str().unwrap()];
        let mut visited = HashSet::new();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            let name = by_id[id]["name"].as_str().unwrap();
            if name.starts_with("sqlx")
                || name.contains("sqlite")
                || name.starts_with("ratatui")
                || name == "crossterm"
                || name == "tokio"
                || name == "git2"
                || name == "portable-pty"
                || name == "vt100"
                || name == "relayterm-tui"
                || name == "relayterm-daemon"
                || name == "relayterm-cli"
                || name == "relayterm-git"
                || name == "relayterm-agent-adapters"
                || name == "relayterm-pty"
            {
                return Err(format!("Forbidden core closure {root} -> {name}"));
            }
            for dep in edges[id]["dependencies"].as_array().unwrap() {
                pending.push(dep.as_str().unwrap());
            }
        }
    }
    Ok(())
}

fn metadata() -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--locked", "--offline"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "cargo metadata failed");
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn workspace_preserves_dependency_direction() {
    validate(&metadata()).unwrap();
}

#[test]
fn rejects_target_specific_internal_edge() {
    let mut graph = metadata();
    let domain = graph["packages"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["name"] == "relayterm-domain")
        .unwrap();
    domain["dependencies"].as_array_mut().unwrap().push(json!({
        "name": "relayterm-tui", "rename": "hidden_ui", "target": "cfg(windows)"
    }));
    assert!(
        validate(&graph)
            .unwrap_err()
            .contains("Forbidden dependency")
    );
}

#[test]
fn rejects_transitive_adapter_in_core() {
    let mut graph = metadata();
    let fake_id = "synthetic-sqlite";
    graph["packages"].as_array_mut().unwrap().push(json!({
        "id": fake_id, "name": "libsqlite3-sys", "dependencies": []
    }));
    let uuid_id = graph["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "uuid")
        .unwrap()["id"]
        .clone();
    let nodes = graph["resolve"]["nodes"].as_array_mut().unwrap();
    nodes.iter_mut().find(|n| n["id"] == uuid_id).unwrap()["dependencies"]
        .as_array_mut()
        .unwrap()
        .push(json!(fake_id));
    nodes.push(json!({"id": fake_id, "dependencies": []}));
    assert!(
        validate(&graph)
            .unwrap_err()
            .contains("Forbidden core closure")
    );
}
