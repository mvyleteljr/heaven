//! Free, separate-process verification of an existing live proof database.
use anyhow::{Result, ensure};
use frontier::{Run, Store, construction, tools};
use serde_json::{Value, json};

#[test]
#[ignore = "Requires FRONTIER_SAVED_PROOF and FRONTIER_SAVED_DB plus optional Onto bundle"]
fn saved_live_generation_reopens_replays_receipts_and_forks() -> Result<()> {
    let report: Value =
        serde_json::from_slice(&std::fs::read(std::env::var("FRONTIER_SAVED_PROOF")?)?)?;
    ensure!(report["passed"] == true, "Require a completed live proof");
    let exported: Run = serde_json::from_value(report["run"].clone())?;
    let store = Store::open(&std::env::var("FRONTIER_SAVED_DB")?)?;
    let mut restored = store.load(&exported.id)?;
    assert_eq!(restored.checkpoint_hash(), exported.checkpoint_hash());
    assert_eq!(
        restored.checkpoint_hash(),
        report["audit"]["checkpoint_hash"]
    );
    let keys: Vec<_> = restored
        .world
        .ontology
        .as_ref()
        .unwrap()
        .modules
        .keys()
        .cloned()
        .collect();
    assert!(!keys.is_empty());
    for key in &keys {
        let read = tools::call(
            "http.request",
            json!({"method":"GET","url":format!("https://institutions.local/modules/{key}")}),
        );
        assert!(construction::direct(&restored, &read)?.request.is_none());
        let expected = report["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["label"] == format!("{key}/accept-after"))
            .unwrap()["actual"]
            .clone();
        let before = restored.world.ontology.clone();
        let command = json!({"command_id":"after","story":"Operator creates WorkItem for Policy","participants":{"Policy":"Policy:main"},"fields":{"amount":20}});
        let actual = restored.world.ontology.as_mut().unwrap().request(
            "POST",
            &format!("/modules/{key}/commands"),
            Some(&command.to_string()),
            0,
        )?;
        assert_eq!(actual, expected);
        assert_eq!(restored.world.ontology, before);
        let mut branch = restored.fork(restored.seq);
        let mut new_command = command.clone();
        new_command["command_id"] = json!("cold-fork");
        new_command["fields"]["amount"] = json!(7);
        let result = branch.world.ontology.as_mut().unwrap().request(
            "POST",
            &format!("/modules/{key}/commands"),
            Some(&new_command.to_string()),
            0,
        )?;
        assert_eq!(result["committed"], true);
        assert_ne!(branch.world.ontology, before);
        assert_eq!(restored.world.ontology, before);
    }
    assert_eq!(
        store.load(&exported.id)?.checkpoint_hash(),
        exported.checkpoint_hash()
    );
    println!(
        "{}",
        json!({"run_id":exported.id,"modules":keys,"checkpoint_hash":restored.checkpoint_hash(),"cold_reopen":true,"receipt_replay":true,"fork_execution":true,"parent_unchanged":true,"inference_calls":0})
    );
    Ok(())
}
