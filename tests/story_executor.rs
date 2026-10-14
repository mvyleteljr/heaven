use anyhow::Result;
use frontier::{
    Config, Run, Store, construction, experiments, institutions, story_executor, tools,
};
use serde_json::{Value, json};

fn setup() -> Result<Run> {
    let preview = experiments::institutions::preview("story-review")?;
    assert_eq!(
        preview["runtime_available"], true,
        "{}",
        preview["runtime_error"]
    );
    let mut run = Run::new(Config::default())?;
    run.world = serde_json::from_value(preview["world"].clone())?;
    run.experiment = Some(experiments::Experiment::OntoReview);
    Ok(run)
}

fn execute(module: &str, id: &str, story: &str, extra: Value) -> frontier::Action {
    let mut inputs = json!({"module":module,"command_id":id,"story":story});
    inputs
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    tools::call("onto.execute", inputs)
}

#[test]
fn story_input_validation_rejects_identity_injection_and_invalid_bindings() {
    for inputs in [
        json!({"module":"../bad"}),
        json!({"module":"payments/facts"}),
        json!({"module":"payments","source":"pretend approved"}),
    ] {
        assert!(tools::validate_action(&tools::call("onto.expand", inputs)).is_err());
    }
    for extra in [
        json!({"actor":"Operator:Admin"}),
        json!({"fields_json":"[]"}),
        json!({"participants_json":"{\"Policy\":4}"}),
    ] {
        assert!(
            tools::validate_action(&execute("claims", "test", "Operator expands Policy", extra))
                .is_err()
        );
    }
}

#[test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-onto"]
fn story_execution_compiles_additions_and_preserves_receipts_state_and_boundaries() -> Result<()> {
    let mut run = setup()?;
    let foundation = run.world.ontology.as_ref().unwrap().snapshot.clone();
    for action in [
        tools::call("shell.exec", json!({"command":"echo bypass"})),
        tools::call(
            "http.request",
            json!({"method":"POST","url":"https://institutions.local/commands","body":"{}"}),
        ),
        tools::call(
            "file.write",
            json!({"path":"world.json","content":"approved"}),
        ),
        tools::call(
            "mail.send",
            json!({"to":"customer","subject":"approved","body":"paid"}),
        ),
    ] {
        let before = run.world.clone();
        assert!(matches!(
            construction::direct(&run, &action)?.resolution,
            construction::Resolution::Denied
        ));
        assert!(run.world.apply(&action).is_err());
        assert_eq!(run.world, before);
    }
    let expand = tools::call("onto.expand", json!({"module":"claims"}));
    let request = construction::direct(&run, &expand)?.request.unwrap();
    let source = include_str!("../examples/jit/test-capability.onto");
    let invalid = institutions::proposal(
        &request,
        &json!({"source":"object Broken\n  value: invented\n","agent_state":{"fields":[]},"error":null}).to_string(),
    )?;
    let before = run.world.clone();
    assert!(construction::publish(&mut run, &request, &invalid).is_err());
    assert_eq!(run.world, before);
    let proposal = institutions::proposal(
        &request,
        &json!({"source":source,"agent_state":{"fields":[]},"error":null}).to_string(),
    )?;
    construction::publish(&mut run, &request, &proposal)?;
    assert_eq!(run.world.apply(&expand)?["module"], "claims");
    assert!(construction::direct(&run, &expand)?.request.is_none());
    let read = tools::call("onto.inspect", json!({"module":"claims","view":"model"}));
    let model = run.world.apply(&read)?;
    assert!(
        model["stories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["story"] == "Operator expands Policy")
    );
    assert!(!model.to_string().contains("http.request"));

    let work = |id| {
        execute(
            "claims",
            id,
            "Operator creates WorkItem for Policy",
            json!({"participants_json":"{\"Policy\":\"Policy:main\"}","fields_json":"{\"amount\":20}"}),
        )
    };
    let initial = run.world.ontology.as_ref().unwrap().modules["claims"]
        .snapshot
        .clone();
    let refused = run.world.apply(&work("before"))?;
    assert_eq!(refused["committed"], false);
    assert_eq!(refused["changes"], json!([]));
    assert_eq!(
        run.world.ontology.as_ref().unwrap().modules["claims"].snapshot["instances"],
        initial["instances"]
    );
    let change = run.world.apply(&execute(
        "claims",
        "policy",
        "Operator expands Policy",
        json!({"object":"Policy:main"}),
    ))?;
    assert_eq!(change["committed"], true);
    assert_eq!(change["changes"][0]["id"], "Policy:main");
    assert_eq!(change["changes"][0]["before"]["fields"]["limit"], 10);
    assert_eq!(change["changes"][0]["after"]["fields"]["limit"], 50);
    let accepted = run.world.apply(&work("after"))?;
    assert_eq!(accepted["committed"], true);
    let state = run.world.ontology.clone();
    let replay = run.world.apply(&work("after"))?;
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["changes"], json!([]));
    assert_eq!(run.world.ontology, state);
    assert_eq!(run.world.ontology.as_ref().unwrap().snapshot, foundation);
    let receipts = run.world.apply(&tools::call(
        "onto.inspect",
        json!({"module":"claims","view":"receipts"}),
    ))?;
    assert_eq!(receipts["receipts"].as_object().unwrap().len(), 3);
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("review.sqlite");
    let mut store = Store::open(path.to_str().unwrap())?;
    store.commit(&mut run, "outcome", "Story evidence", accepted)?;
    drop(store);
    let saved = Store::open(path.to_str().unwrap())?.load(&run.id)?;
    assert_eq!(saved.world, run.world);
    assert_eq!(saved.experiment, Some(experiments::Experiment::OntoReview));

    let mut ordinary = experiments::institutions::world(
        experiments::institutions::definition("accounts")?,
        "Transfer",
    )?;
    assert!(!story_executor::enabled(&ordinary));
    assert!(
        ordinary
            .apply(&tools::call("onto.inspect", json!({"view":"overview"})))
            .is_err()
    );
    assert!(
        ordinary
            .apply(&tools::call(
                "http.request",
                json!({"method":"GET","url":"https://institutions.local/model"})
            ))
            .is_ok()
    );
    Ok(())
}
