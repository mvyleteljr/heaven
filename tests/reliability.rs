use anyhow::Result;
use frontier::{Config, Run, Store, experiments, tools, workspace::Workspace};
use serde_json::json;

#[test]
fn stale_commits_never_rewind_events_or_mutate_the_caller() -> Result<()> {
    let mut store = Store::open(":memory:")?;
    let mut run = Run::new(Config::default())?;
    store.commit(&mut run, "seed", "seed", json!({}))?;
    let mut stale = run.clone();
    run.actions = 1;
    store.commit(&mut run, "outcome", "new", json!({}))?;
    let before = serde_json::to_value(&stale)?;
    assert!(
        store
            .commit(&mut stale, "outcome", "old", json!({}))
            .is_err()
    );
    assert_eq!(serde_json::to_value(&stale)?, before);
    assert_eq!(store.load(&run.id)?.actions, 1);
    assert_eq!(store.events(&run.id)?.len(), 2);
    Ok(())
}

#[test]
fn context_setting_preserves_legacy_serialization_and_branches() -> Result<()> {
    let legacy = serde_json::to_value(Config::default())?;
    assert!(legacy.get("context_limit_bytes").is_none());
    let mut config: Config = serde_json::from_value(legacy.clone())?;
    assert_eq!(config.context_limit_bytes, 40_000);
    assert_eq!(serde_json::to_value(&config)?, legacy);
    config.context_limit_bytes = 256_000;
    let run = Run::new(config)?;
    assert_eq!(run.fork(1).config.context_limit_bytes, 256_000);
    for limit in [0, 39999, 1_048_577, usize::MAX] {
        let mut invalid = run.config.clone();
        invalid.context_limit_bytes = limit;
        assert!(invalid.validate().is_err());
    }
    Ok(())
}

#[test]
fn failed_construction_uses_experiment_time_and_ends_at_horizon() {
    let mut world = experiments::bellwether::seed();
    let action = tools::call(
        "http.request",
        json!({"method":"GET","url":"https://bellwether.local/organizations/quay/about"}),
    );
    tools::advance_action_time(&mut world, &action);
    assert_eq!(world.tools.as_ref().unwrap().elapsed_seconds, 60);
    tools::advance_time(&mut world, frontier::business::HORIZON - 61);
    tools::advance_action_time(&mut world, &action);
    assert!(world.finished);
    assert_eq!(
        world.tools.unwrap().elapsed_seconds,
        frontier::business::HORIZON
    );
}

#[test]
fn workspace_rejects_unrepresentable_paths_and_rolls_back_quota_failures() -> Result<()> {
    let mut workspace = Workspace::default();
    for name in ["bad\0path", "../escape", "/etc/passwd", ".", "/workspace/"] {
        assert!(workspace.write(name, "test").is_err());
    }
    workspace.write("file", "original")?;
    assert!(workspace.write("file/child", "no").is_err());
    assert!(workspace.write("file", &"x".repeat(1_048_577)).is_err());
    for i in 1..128 {
        workspace.write(&format!("file-{i}"), "x")?;
    }
    let before = workspace.clone();
    assert!(workspace.write("nested/overflow", "no").is_err());
    assert_eq!(workspace, before);
    assert_eq!(workspace.read("file", 0, 100)?["content"], "original");
    Ok(())
}

#[test]
fn sandbox_reaps_background_writers_and_bounds_both_output_streams() -> Result<()> {
    let mut workspace = Workspace::default();
    let started = std::time::Instant::now();
    let result = workspace.exec("(sleep 20; echo late > late) & echo done")?;
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
    assert_eq!(result["stdout"], "done\n");
    assert!(!workspace.files.contains_key("late"));
    let result = workspace
        .exec("/bin/busybox yes x | head -c 50000; /bin/busybox yes y | head -c 50000 >&2")?;
    assert_eq!(result["truncated"], true);
    assert_eq!(result["stdout"].as_str().unwrap().len(), 32768);
    assert_eq!(result["stderr"].as_str().unwrap().len(), 32768);
    Ok(())
}
