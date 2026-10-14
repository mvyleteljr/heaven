use anyhow::Result;
use frontier::{
    Config, Run, Store,
    construction::{self, Outcome, Proposal, Resource},
    experiments::Experiment,
    institutions, ontology, tools,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::Instant;

fn seeded() -> Result<Run> {
    let mut run = Run::new(Config::default())?;
    run.experiment = Some(Experiment::Institutions);
    run.world.tools = Some(tools::State::empty());
    let mut state = ontology::State::initialize(ontology::Definition {
        source: BTreeMap::from([(
            "foundation.onto".into(),
            include_str!("../examples/jit/foundation.onto").into(),
        )]),
        actor: "Operator:Ada".into(),
        epoch: "2026-01-01T00:00:00Z".into(),
        public_facts: true,
        agent_state: None,
    })?;
    institutions::enable(
        &mut state,
        institutions::Policy {
            brief: "A company with capabilities generated on demand.".into(),
            max_modules: 128,
            repair_attempts: 0,
        },
    )?;
    run.world.ontology = Some(state);
    Ok(run)
}
fn request(run: &Run, key: &str) -> Result<construction::Request> {
    Ok(construction::direct(
        run,
        &tools::call(
            "http.request",
            json!({"method":"GET","url":format!("https://institutions.local/modules/{key}")}),
        ),
    )?
    .request
    .unwrap())
}
fn proposal(request: &construction::Request, source: &str) -> Proposal {
    Proposal {
        version: 1,
        request_id: request.request_id.clone(),
        basis: request.basis.clone(),
        result: Outcome::Proposed {
            resources: vec![Resource {
                target: request.directive.target.clone(),
                operation: "create".into(),
                initial_state: json!({"source":source}),
                interface: request.interface.clone(),
                functionality: "onto.module.v1".into(),
                dependencies: vec![ontology::ROOT.into()],
            }],
        },
    }
}
fn command(run: &mut Run, key: &str, input: Value) -> Result<Value> {
    run.world.ontology.as_mut().unwrap().request(
        "POST",
        &format!("/modules/{key}/commands"),
        Some(&input.to_string()),
        0,
    )
}
fn work(id: &str, amount: i64) -> Value {
    json!({"command_id":id,"story":"Operator creates WorkItem for Policy","participants":{"Policy":"Policy:main"},"fields":{"amount":amount}})
}

#[test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-jit"]
fn generation_validates_hidden_foundation_stories_before_enabling() -> Result<()> {
    let source = include_str!("../examples/jit/foundation.onto");
    let mut state = ontology::State::initialize(ontology::Definition {
        source: BTreeMap::from([(
            "foundation.onto".into(),
            format!(
                "{source}\nactor System\n  @type system\n\nstory Observe about Company\n  System logs Company:notice\n"
            ),
        )]),
        actor: "Operator:Ada".into(),
        epoch: "2026-01-01T00:00:00Z".into(),
        public_facts: true,
        agent_state: None,
    })?;
    assert_eq!(
        state.stories,
        json!([]),
        "System Stories are hidden from actor docs"
    );
    let original = state.clone();
    let policy = institutions::Policy {
        brief: "Discover company capabilities.".into(),
        max_modules: 4,
        repair_attempts: 0,
    };
    assert!(
        institutions::enable(&mut state, policy.clone())
            .unwrap_err()
            .to_string()
            .contains("must have no Stories")
    );
    assert_eq!(
        state, original,
        "Invalid foundations must not be enabled partially"
    );

    let mut state = ontology::State::initialize(ontology::Definition {
        source: BTreeMap::from([(
            "foundation.onto".into(),
            format!("{source}\nobject Reserved\n  name: string @required\n"),
        )]),
        actor: "Operator:Ada".into(),
        epoch: "2026-01-01T00:00:00Z".into(),
        public_facts: true,
        agent_state: None,
    })?;
    institutions::enable(&mut state, policy)?;
    assert!(state.protected_types.contains(&"Reserved".to_string()));
    assert_eq!(json!(state.protected), state.snapshot["instances"]);
    Ok(())
}

#[test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-jit"]
fn generated_behavior_restructures_and_survives_restart_fork_and_adversarial_proposals()
-> Result<()> {
    let mut run = seeded()?;
    let directory = run
        .world
        .ontology
        .as_mut()
        .unwrap()
        .request("GET", "/", None, 0)?;
    assert!(directory.get("generation").is_none());
    assert!(directory.get("module_count").is_none());
    assert!(
        directory["module_discovery"]
            .as_str()
            .unwrap()
            .contains("/modules/NAME")
    );
    assert!(!directory.to_string().contains("generated on demand"));
    // The researcher-owned brief remains available to the generator only.
    assert_eq!(
        institutions::context(&run)?["brief"],
        "A company with capabilities generated on demand."
    );
    let source = include_str!("../examples/jit/test-capability.onto");
    for url in [
        "https://institutions.local/modules/UPPER",
        "https://institutions.local/modules/claims/facts",
        "https://institutions.local/modules/claims?authority=admin",
        "https://institutions.local/modules/claims/extra",
    ] {
        assert!(
            construction::direct(
                &run,
                &tools::call("http.request", json!({"method":"GET","url":url}))
            )?
            .request
            .is_none()
        );
    }
    let req = request(&run, "claims")?;
    let mut wrong_scope = proposal(&req, source);
    if let Outcome::Proposed { resources } = &mut wrong_scope.result {
        resources[0].target = "https://institutions.local/modules/other".into();
    }
    let before = run.world.clone();
    assert!(construction::publish(&mut run, &req, &wrong_scope).is_err());
    assert_eq!(run.world, before);
    for bad in [
        "object Invalid\n  f: madeuptype\n".to_owned(),
        format!("{source}\nseed Company\n  name: \"Counterfeit\", ceiling: 100\n"),
        format!("{source}\nobject Company\n  ceiling: int\n"),
        source.replace("name: \"main\", limit: 10", "name: \"main\", limit: -1"),
    ] {
        assert!(construction::publish(&mut run, &req, &proposal(&req, &bad)).is_err());
        assert_eq!(run.world, before);
    }
    construction::publish(&mut run, &req, &proposal(&req, source))?;
    let published = run.world.clone();
    assert!(construction::publish(&mut run, &req, &proposal(&req, source)).is_err());
    assert_eq!(run.world, published);
    let read = tools::call(
        "http.request",
        json!({"method":"GET","url":"https://institutions.local/modules/claims"}),
    );
    assert!(construction::direct(&run, &read)?.request.is_none());
    let initial = run.world.ontology.as_ref().unwrap().modules["claims"]
        .snapshot
        .clone();
    assert_eq!(
        command(&mut run, "claims", work("before", 20))?["committed"],
        false
    );
    assert_eq!(
        run.world.ontology.as_ref().unwrap().modules["claims"].snapshot,
        initial
    );
    assert_eq!(
        command(
            &mut run,
            "claims",
            json!({"command_id":"restructure","story":"Operator expands Policy","object":"Policy:main"})
        )?["committed"],
        true
    );
    let accepted = command(&mut run, "claims", work("after", 20))?;
    assert_eq!(accepted["committed"], true);
    assert_eq!(
        command(&mut run, "claims", work("over-limit", 101))?["committed"],
        false
    );
    let after = run.world.ontology.as_ref().unwrap().modules["claims"]
        .snapshot
        .clone();
    assert_eq!(command(&mut run, "claims", work("after", 20))?, accepted);
    assert_eq!(
        run.world.ontology.as_ref().unwrap().modules["claims"].snapshot,
        after
    );
    assert_eq!(
        after["instances"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["type"] == "WorkItem")
            .count(),
        1
    );
    assert_eq!(
        run.world.ontology.as_ref().unwrap().snapshot,
        before.ontology.as_ref().unwrap().snapshot
    );

    // A generated Story can parse and compile but still has no authority to
    // mutate foundation types. Refusal is atomic at the runtime boundary.
    let evil = format!(
        "{source}\nstory Attack\n  Operator changes Company\n    sets Company.ceiling to 999\n"
    );
    let req = request(&run, "malicious")?;
    construction::publish(&mut run, &req, &proposal(&req, &evil))?;
    let intact = run.world.ontology.as_ref().unwrap().modules["malicious"].clone();
    assert!(command(&mut run,"malicious",json!({"command_id":"attack","story":"Operator changes Company","object":"Company:Core"})).is_err());
    assert_eq!(
        run.world.ontology.as_ref().unwrap().modules["malicious"],
        intact
    );
    let mut forged = work("forged", 1);
    forged["actor"] = json!("Operator:Other");
    assert!(command(&mut run, "claims", forged).is_err());

    let dir = tempfile::tempdir()?;
    let path = dir.path().join("proof.sqlite");
    let mut store = Store::open(path.to_str().unwrap())?;
    store.commit(&mut run, "seed", "proof", json!({}))?;
    let hash = run.checkpoint_hash();
    drop(store);
    let store = Store::open(path.to_str().unwrap())?;
    let mut restored = store.load(&run.id)?;
    assert_eq!(restored.checkpoint_hash(), hash);
    assert_eq!(
        command(&mut restored, "claims", work("after", 20))?,
        accepted
    );
    assert!(construction::direct(&restored, &read)?.request.is_none());
    let mut branch = restored.fork(restored.seq);
    assert_eq!(
        command(&mut branch, "claims", work("branch", 7))?["committed"],
        true
    );
    assert_eq!(
        restored.world.ontology.as_ref().unwrap().modules["claims"].snapshot,
        after
    );
    assert_ne!(
        branch.world.ontology.as_ref().unwrap().modules["claims"].snapshot,
        after
    );
    Ok(())
}

#[test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-jit"]
fn growth_keeps_generator_context_bounded_and_measures_latency() -> Result<()> {
    let mut run = seeded()?;
    let mut measurements = vec![];
    let initial = context_size(&run)?;
    for n in 0..64 {
        let key = format!("capability-{n}");
        let started = Instant::now();
        let req = request(&run, &key)?;
        construction::publish(
            &mut run,
            &req,
            &proposal(&req, include_str!("../examples/jit/test-capability.onto")),
        )?;
        let publish_ms = started.elapsed().as_millis();
        let start = Instant::now();
        command(&mut run, &key, work("one", 1))?;
        let execute_ms = start.elapsed().as_millis();
        let start = Instant::now();
        run.world.ontology.as_mut().unwrap().request(
            "GET",
            "/modules/capability-0/model",
            None,
            0,
        )?;
        measurements.push(json!({"modules":n+1,"publish_ms":publish_ms,"execute_ms":execute_ms,"revisit_ms":start.elapsed().as_millis(),"context_bytes":context_size(&run)?}));
        assert_eq!(context_size(&run)?, initial);
    }
    let first = run.world.ontology.as_ref().unwrap().modules["capability-0"].clone();
    let req = request(&run, "one-more")?;
    construction::publish(
        &mut run,
        &req,
        &proposal(&req, include_str!("../examples/jit/test-capability.onto")),
    )?;
    assert_eq!(
        run.world.ontology.as_ref().unwrap().modules["capability-0"],
        first
    );
    println!(
        "JIT_GROWTH_REPORT={}",
        json!({"kind":"deterministic_fixture_not_live_generation","measurements":measurements})
    );
    Ok(())
}
fn context_size(run: &Run) -> Result<usize> {
    Ok(serde_json::to_vec(&institutions::context(run)?)?.len())
}

#[test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-jit"]
fn generator_reference_lifecycle_and_timestamp_example_executes() -> Result<()> {
    let mut run = seeded()?;
    let req = request(&run, "reference")?;
    construction::publish(
        &mut run,
        &req,
        &proposal(&req, include_str!("../src/onto_example.onto")),
    )?;
    let created = command(
        &mut run,
        "reference",
        json!({"command_id":"create","story":"Operator creates Ticket","fields":{"name":"example"}}),
    )?;
    assert_eq!(created["committed"], true);
    assert_eq!(
        command(
            &mut run,
            "reference",
            json!({"command_id":"close","story":"Operator closes Ticket","object":created["created"]["id"]})
        )?["committed"],
        true
    );
    Ok(())
}

#[test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-jit"]
fn agent_state_shape_is_validated_atomically_with_module_publication() -> Result<()> {
    let mut base = seeded()?;
    let state = base.world.ontology.as_mut().unwrap();
    let shape = frontier::agent_state::Declaration::identity(&state.definition.actor);
    state.agent_state = Some(shape.project(&state.definition.actor, &state.snapshot)?);
    state.definition.agent_state = Some(shape);
    let source = include_str!("../examples/jit/test-agent-memory.onto");
    let valid = json!({"fields":[{"key":"last_observation","label":"Last observation","kind":"string","format":"plain","source":{"instance":"AgentMemory:main","field":"lastObservation"}}]});
    let mut wrong_type = valid.clone();
    wrong_type["fields"][0]["kind"] = json!("integer");
    let mut missing = valid.clone();
    missing["fields"][0]["source"]["field"] = json!("invented");
    let mut foundation = valid.clone();
    foundation["fields"][0]["source"] = json!({"instance":"Company:Core","field":"name"});
    let mut duplicate = valid.clone();
    duplicate["fields"]
        .as_array_mut()
        .unwrap()
        .push(valid["fields"][0].clone());
    for invalid in [Value::Null, wrong_type, missing, foundation, duplicate] {
        let mut run = base.clone();
        let request = request(&run, "repair-intake")?;
        let mut candidate = proposal(&request, source);
        let Outcome::Proposed { resources } = &mut candidate.result else {
            unreachable!()
        };
        resources[0].initial_state["agent_state"] = invalid;
        assert!(construction::publish(&mut run, &request, &candidate).is_err());
        assert_eq!(
            run.world, base.world,
            "Invalid shapes cannot partially publish state"
        );
    }
    let request = request(&base, "repair-intake")?;
    let mut candidate = proposal(&request, source);
    let Outcome::Proposed { resources } = &mut candidate.result else {
        unreachable!()
    };
    resources[0].initial_state["agent_state"] = valid;
    construction::publish(&mut base, &request, &candidate)?;
    let module = &base.world.ontology.as_ref().unwrap().modules["repair-intake"];
    assert_eq!(
        module.agent_state.as_ref().unwrap()["values"]["last_observation"],
        "unobserved"
    );
    Ok(())
}
