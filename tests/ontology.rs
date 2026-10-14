use anyhow::Result;
use frontier::{Config, Run, Store, ontology, tools};
use serde_json::{Value, json};
const STORY: &str = "Clerk creates Transfer from Account:sender to Account:recipient";
fn transfer(key: &str, reference: &str, amount: i64, sender: &str) -> Value {
    json!({"command_id":key,"story":STORY,"participants":{"Account:sender":sender,"Account:recipient":"Account:Reserve"},"fields":{"amount":amount,"reference":reference}})
}
fn balances(state: &ontology::State) -> Vec<i64> {
    state.snapshot["instances"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["type"] == "Account")
        .map(|v| v["fields"]["balance"].as_i64().unwrap())
        .collect()
}

#[test]
#[ignore = "Requires the pinned optional Onto bundle; scripts/dev check-onto runs this test"]
fn onto_is_the_only_ledger_and_commands_survive_restart_without_duplicate_effects() -> Result<()> {
    let definition = ontology::Definition {
        source: std::collections::BTreeMap::from([(
            "accounts.onto".into(),
            include_str!("../examples/onto/accounts.onto").into(),
        )]),
        actor: "Clerk:Ada".into(),
        epoch: "2026-01-01T00:00:00Z".into(),
        public_facts: true,
    };
    let mut state = ontology::State::initialize(definition)?;
    assert_eq!(balances(&state), vec![10000, 5000, 2500]);
    let command = transfer("one", "transfer-1", 2500, "Account:Operating").to_string();
    let result = state.request("POST", "/commands", Some(&command), 10)?;
    assert_eq!(result["committed"], true);
    assert_eq!(balances(&state), vec![7500, 7500, 2500]);
    assert_eq!(
        state.request("POST", "/commands", Some(&command), 20)?,
        result
    );
    assert_eq!(state.receipts.len(), 1);
    let before = state.snapshot.clone();
    for (key, reference, amount, sender) in [
        ("overdraft", "transfer-2", 7501, "Account:Operating"),
        ("unauthorized", "transfer-3", 1, "Account:External"),
        ("negative", "transfer-4", -1, "Account:Operating"),
        ("duplicate", "transfer-1", 1, "Account:Operating"),
    ] {
        let refused = state.request(
            "POST",
            "/commands",
            Some(&transfer(key, reference, amount, sender).to_string()),
            10,
        )?;
        assert_eq!(refused["committed"], false, "{refused}");
        assert_eq!(state.snapshot, before);
    }
    let before = state.clone();
    assert!(
        state
            .request(
                "POST",
                "/commands",
                Some(&transfer("one", "other", 10, "Account:Operating").to_string()),
                10
            )
            .is_err()
    );
    let mut forged = transfer("forged", "transfer-5", 1, "Account:External");
    forged["actor"] = json!("Clerk:Visitor");
    assert!(
        state
            .request("POST", "/commands", Some(&forged.to_string()), 10)
            .is_err()
    );
    assert_eq!(state, before);
    let mut run = Run::new(Config::default())?;
    run.world.ontology = Some(state);
    run.world.tools = Some(tools::State::empty());
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("onto.sqlite");
    let mut store = Store::open(path.to_str().unwrap())?;
    store.commit(&mut run, "seed", "institution", json!({}))?;
    let hash = run.checkpoint_hash();
    drop(store);
    let store = Store::open(path.to_str().unwrap())?;
    let mut restored = store.load(&run.id)?;
    assert_eq!(restored.checkpoint_hash(), hash);
    assert_eq!(
        restored.world.ontology.as_mut().unwrap().request(
            "POST",
            "/commands",
            Some(&command),
            30
        )?,
        result
    );
    let mut branch = restored.fork(restored.seq);
    let branch_command = transfer("branch", "transfer-branch", 500, "Account:Operating");
    branch.world.ontology.as_mut().unwrap().request(
        "POST",
        "/commands",
        Some(&branch_command.to_string()),
        30,
    )?;
    assert_eq!(
        balances(restored.world.ontology.as_ref().unwrap()),
        vec![7500, 7500, 2500]
    );
    assert_eq!(
        balances(branch.world.ontology.as_ref().unwrap()),
        vec![7000, 8000, 2500]
    );
    assert_eq!(
        balances(branch.world.ontology.as_ref().unwrap())
            .iter()
            .sum::<i64>(),
        17500
    );
    assert!(restored.world.business.is_none());
    let state = branch.world.ontology.as_mut().unwrap();
    state.runtime_sha256 = "wrong-runtime".into();
    assert!(state.check_runtime().is_err());
    assert!(state.request("GET", "/facts", None, 40).is_err());
    Ok(())
}
