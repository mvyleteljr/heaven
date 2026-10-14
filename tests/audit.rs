use anyhow::Result;
use frontier::{Config, Run, Store, audit, experiments, tools};
use serde_json::json;

#[test]
fn refusal_and_repetition_are_measured_without_changing_outcome() -> Result<()> {
    let mut run = Run::new(Config::default())?;
    run.world = experiments::records::seed();
    run.experiment = Some(experiments::Experiment::Records);
    let mut store = Store::open(":memory:")?;
    store.commit(&mut run, "seed", "records", json!({}))?;
    for _ in 0..4 {
        let action = tools::call(
            "http.request",
            json!({"method":"GET","url":"https://records.local/absent"}),
        );
        let result = run.world.apply(&action)?;
        assert_eq!(result["status"], 404);
        run.world
            .transcript
            .push(json!({"action":action,"result":result}));
        run.actions += 1;
        store.commit(
            &mut run,
            "outcome",
            "refusal",
            json!({"action":action,"result":result}),
        )?;
    }
    let before = run.checkpoint_hash();
    let report = audit::report(&run, &store.events(&run.id)?);
    assert_eq!(report["behavior"]["failed_tool_results"], 4);
    assert_eq!(report["behavior"]["repeated_actions_after_first"], 3);
    assert_eq!(report["behavior"]["longest_identical_action_streak"], 4);
    assert_eq!(report["outcome"]["kind"], "unscored");
    assert_eq!(report["failures"], json!([]));
    assert_eq!(run.checkpoint_hash(), before);
    let mut branch = run.fork(run.seq);
    store.commit(&mut branch, "fork", "branch", json!({}))?;
    let branch_report = audit::report(&branch, &store.events(&branch.id)?);
    assert_eq!(branch_report["integrity"]["events_contiguous"], true);
    assert_eq!(branch_report["integrity"]["completed_observations"], 4);
    Ok(())
}

#[test]
fn wrong_or_early_handoff_is_an_experiment_result_not_an_integrity_failure() -> Result<()> {
    let mut run = Run::new(Config::default())?;
    run.world = experiments::recall::seed();
    run.experiment = Some(experiments::Experiment::Recall);
    let mut store = Store::open(":memory:")?;
    store.commit(&mut run, "seed", "recall", json!({}))?;
    run.world.apply(&tools::call(
        "file.write",
        json!({"path":"handoff.json","content":"{\"reference_code\":\"AT-17\"}"}),
    ))?;
    run.world.finished = true;
    run.status = "finished".into();
    store.commit(&mut run,"outcome","early",json!({"action":{"kind":"file.write","inputs":{"path":"handoff.json"}},"elapsed_seconds":1}))?;
    let report = audit::report(&run, &store.events(&run.id)?);
    assert_eq!(report["status"], "finished");
    assert_eq!(report["integrity"]["events_contiguous"], true);
    assert_eq!(report["outcome"]["task_complete"], false);
    assert_eq!(report["outcome"]["no_early_submission"], false);
    assert_eq!(report["failures"], json!([]));
    Ok(())
}
