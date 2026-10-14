use anyhow::Result;
use frontier::{
    Config, Run, Store,
    construction::*,
    experiments::{Experiment, records},
    tools,
};
use serde_json::{Value, json};

fn action(target: &str) -> frontier::Action {
    tools::call(
        "http.request",
        json!({"method":"GET","url":format!("https://records.local/{target}")}),
    )
}
fn proposal(request: &Request, data: Value) -> Proposal {
    Proposal {
        version: 1,
        request_id: request.request_id.clone(),
        basis: request.basis.clone(),
        result: Outcome::Proposed {
            resources: vec![Resource {
                operation: "create".into(),
                target: request.directive.target.clone(),
                initial_state: data,
                interface: request.interface.clone(),
                functionality: "engine.static-http.v1".into(),
                dependencies: serde_json::from_value(
                    request.directive.requirements["dependencies"].clone(),
                )
                .unwrap(),
            }],
        },
    }
}
fn seed() -> Result<Run> {
    let mut run = Run::new(Config::default())?;
    run.experiment = Some(Experiment::Records);
    run.world = records::seed();
    Ok(run)
}
fn ledger() -> Value {
    json!({"title":"Invoices","invoices":[
        {"id":"A","vendor":"Oak","amount_cents":12347,"description":"Paper"},
        {"id":"B","vendor":"Ash","amount_cents":45679,"description":"Chairs"},
        {"id":"C","vendor":"Elm","amount_cents":89123,"description":"Repairs"}]})
}

#[test]
fn publication_rejects_invalid_proposals_without_mutation() -> Result<()> {
    let mut run = seed()?;
    let request = direct(&run, &action("ledger"))?.request.unwrap();
    let valid = serde_json::to_value(proposal(&request, ledger()))?;
    for (pointer, value) in [
        ("/version", json!(2)),
        ("/request_id", json!("other")),
        ("/basis/revision", json!("old")),
        (
            "/result/resources/0/target",
            json!("https://records.local/other"),
        ),
        ("/result/resources/0/operation", json!("modify")),
        ("/result/resources/0/interface/method", json!("POST")),
        ("/result/resources/0/functionality", json!("generated-code")),
        ("/result/resources/0/dependencies", json!([])),
        (
            "/result/resources/0/initial_state/invoices/0/amount_cents",
            json!(0),
        ),
        (
            "/result",
            json!({"status":"failed","reason":"cannot construct"}),
        ),
    ] {
        let mut invalid = valid.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        let before = serde_json::to_value(&run.world)?;
        assert!(
            publish(&mut run, &request, &serde_json::from_value(invalid)?).is_err(),
            "accepted {pointer}"
        );
        assert_eq!(serde_json::to_value(&run.world)?, before);
    }
    run.world
        .apply(&tools::call("wait", json!({"seconds":1})))?;
    assert!(publish(&mut run, &request, &serde_json::from_value(valid)?).is_err());
    assert!(matches!(
        direct(&run, &action("review"))?.resolution,
        Resolution::Unsupported
    ));
    assert!(direct(&run, &action("not-supported"))?.request.is_none());
    assert!(matches!(
        direct(
            &run,
            &tools::call("file.read", json!({"path":"../private"}))
        )?
        .resolution,
        Resolution::Denied
    ));
    Ok(())
}

#[test]
fn dependent_construction_reuse_branch_and_reopen_preserve_commitments() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("world.sqlite");
    let mut store = Store::open(path.to_str().unwrap())?;
    let mut run = seed()?;
    store.commit(&mut run, "seed", "seed", json!({}))?;
    run.world.goal = "PRIVATE RESEARCH OBJECTIVE".into();
    for (target, data) in [
        ("ledger", ledger()),
        (
            "policy",
            json!({"title":"Policy","refund_basis_points":735,"eligibility":"Unused items returned within 7 days"}),
        ),
    ] {
        let request = direct(&run, &action(target))?.request.unwrap();
        assert!(!serde_json::to_string(&request)?.contains("PRIVATE RESEARCH"));
        let proposal = proposal(&request, data);
        publish(&mut run, &request, &proposal)?;
        store.commit(&mut run, "published", "published", json!(proposal))?;
        assert!(direct(&run, &action(target))?.request.is_none());
        assert!(publish(&mut run, &request, &proposal).is_err());
    }
    let request = direct(&run, &action("review"))?.request.unwrap();
    assert_eq!(request.context.committed.as_object().unwrap().len(), 2);
    let mut data = json!({"title":"Review","invoice_id":"A","refund_cents":908,"explanation":"Unused paper returned after two days"});
    // 12347 * 735 / 10000 = 907.5045: truncate, do not round.
    assert!(publish(&mut run, &request, &proposal(&request, data.clone())).is_err());
    data["refund_cents"] = json!(907);
    publish(&mut run, &request, &proposal(&request, data))?;
    let first = run.world.apply(&action("ledger"))?;
    assert_eq!(first, run.world.apply(&action("ledger"))?);
    assert!(
        !run.world
            .observation()
            .to_string()
            .contains(&request.request_id)
    );
    run.world.apply(&tools::call(
        "file.write",
        json!({"path":"report.txt","content":"907 cents"}),
    ))?;
    store.commit(&mut run, "outcome", "saved report", json!({}))?;
    let hash = run.checkpoint_hash();
    let mut branch = store.checkpoint(&run.id, run.seq)?.fork(run.seq);
    assert_eq!(branch.checkpoint_hash(), hash);
    branch.world.apply(&tools::call(
        "file.write",
        json!({"path":"report.txt","content":"branch only"}),
    ))?;
    assert_ne!(branch.world.tools, run.world.tools);
    drop(store);
    let store = Store::open(path.to_str().unwrap())?;
    let mut restored = store.load(&run.id)?;
    assert_eq!(restored.checkpoint_hash(), hash);
    assert!(direct(&restored, &action("review"))?.request.is_none());
    assert_eq!(restored.world.apply(&action("ledger"))?, first);
    Ok(())
}
