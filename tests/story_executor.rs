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
#[ignore = "Requires optional Onto bundle; scripts/dev check-onto"]
fn harbor_delivery_timestamp_revision_repairs_buyer_return_without_changing_baseline() -> Result<()>
{
    let base = experiments::harbor_review::definition()?;
    let revised = experiments::harbor_review::definition_v2()?;
    assert_eq!(
        base.source["harbor-works.onto"],
        include_str!("../examples/onto/harbor-works.onto")
    );
    let restored = revised.source["harbor-works-review-v2.onto"]
        .lines()
        .filter(|line| {
            !line.starts_with("  @rule Agreement.status is")
                && !line.starts_with("  @rule not Agreement.deliveredAt exists")
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(restored, base.source["harbor-works.onto"]);
    for (definition, expected) in [(base, false), (revised, true)] {
        let mut world = experiments::harbor_review::seed(definition)?;
        if expected {
            for (id, fields) in [
                ("missing-timestamp", json!({})),
                (
                    "future-timestamp",
                    json!({"deliveredAt":"2026-01-11T00:00:00"}),
                ),
                (
                    "before-creation",
                    json!({"deliveredAt":"2026-01-01T00:00:00"}),
                ),
            ] {
                let before = world.ontology.as_ref().unwrap().snapshot.clone();
                let result = world.apply(&tools::call("onto.execute", json!({
                    "command_id":id,"story":"Business delivers Agreement with Service",
                    "object":"Agreement:fixture-authorized",
                    "participants_json":"{\"Service\":\"Service:fixture-directory\"}","fields_json":fields.to_string()})))?;
                assert_eq!(result["committed"], false, "{id}: {result}");
                assert_eq!(result["changes"], json!([]));
                assert_eq!(world.ontology.as_ref().unwrap().snapshot, before);
            }
        }
        let command = tools::call(
            "onto.execute",
            json!({"command_id":"deliver",
            "story":"Business delivers Agreement with Service", "object":"Agreement:fixture-authorized",
            "participants_json":"{\"Service\":\"Service:fixture-directory\"}","fields_json":if expected { Some("{\"deliveredAt\":\"2026-01-10T00:00:00\"}") } else { None }}),
        );
        let result = world.apply(&command)?;
        assert_eq!(result["committed"], true, "{result}");
        let state = world.ontology.as_mut().unwrap();
        let agreement = state.snapshot["instances"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["id"] == "Agreement:fixture-authorized")
            .unwrap();
        if expected {
            assert_eq!(agreement["fields"]["deliveredAt"], "2026-01-10T00:00:00");
            assert_eq!(agreement["fields"]["validationAt"], "2026-01-11T00:00:00");
        } else {
            assert!(agreement["fields"].get("deliveredAt").is_none());
            assert!(agreement["fields"].get("validationAt").is_none());
        }
        // A separate test caller/clock, not an identity or time tool exposed to the actor.
        state.definition.actor = "Business:Quay Hall".into();
        state.definition.agent_state = Some(frontier::agent_state::Declaration::identity(
            "Business:Quay Hall",
        ));
        let command = json!({"command_id":"buyer-return", "story":"Business returns Agreement", "object":"Agreement:fixture-authorized"}).to_string();
        let result = state.request("POST", "/commands", Some(&command), 86400)?;
        assert_eq!(result["committed"], expected, "{result}");
        if !expected {
            assert!(
                result["violations"].to_string().contains("validationAt"),
                "{result}"
            );
        }
    }
    Ok(())
}

#[test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-onto"]
fn harbor_context_keeps_exact_evidence_once_and_blocks_repeated_reads() -> Result<()> {
    let mut run = Run::new(Config::default())?;
    run.world = experiments::harbor_review::seed(experiments::harbor_review::definition_v2()?)?;
    let initial = frontier::memory::observation(&run);
    assert_eq!(
        initial["review_context"]["facts"]["instances"]
            .as_array()
            .unwrap()
            .len(),
        45
    );
    assert_eq!(
        initial["review_context"]["agent_state"]["values"]["available"],
        470000
    );
    assert_eq!(
        initial["review_context"]["agent_state"],
        serde_json::to_value(&run.world.ontology.as_ref().unwrap().agent_state)?
    );
    assert!(initial["history"].is_null());
    assert_eq!(initial["tools"].as_array().unwrap().len(), 2);
    for _ in 0..11 {
        // Legacy saved reads cannot bloat or overwrite the authoritative current snapshot.
        run.world.transcript.push(json!({"action":{"kind":"onto.inspect","inputs":{"view":"facts"}},"result":{"snapshot":initial["review_context"]["facts"]}}));
    }
    let snapshot = run.world.clone();
    assert!(
        run.world
            .apply(&tools::call("onto.inspect", json!({"view":"facts"})))
            .is_err()
    );
    assert_eq!(run.world, snapshot);
    let action = tools::call(
        "onto.execute",
        json!({"command_id":"context-delivery", "story":"Business delivers Agreement with Service", "object":"Agreement:fixture-authorized", "participants_json":"{\"Service\":\"Service:fixture-directory\"}", "fields_json":"{\"deliveredAt\":\"2026-01-10T00:00:00\"}"}),
    );
    let result = run.world.apply(&action)?;
    assert_eq!(result["committed"], true);
    run.world
        .transcript
        .push(json!({"action":action,"result":result}));
    let before = run.checkpoint_hash();
    let observed = frontier::memory::observation(&run);
    assert_eq!(run.checkpoint_hash(), before);
    assert_eq!(
        observed["review_context"]["command_ledger"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let entry = &observed["review_context"]["command_ledger"][0];
    assert_eq!(entry["history_index"], 11);
    assert_eq!(
        entry["result"]["changes"][0]["fields"]["status"]["after"],
        "delivered"
    );
    assert_eq!(
        entry["result"]["changes"][0]["fields"]["deliveredAt"]["before_present"],
        false
    );
    assert!(observed.to_string().len() < initial.to_string().len() + 4000);
    assert_eq!(run.world.transcript.len(), 12);
    Ok(())
}

#[test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-onto"]
fn harbor_review_executes_real_stories_preserves_authority_and_reopens() -> Result<()> {
    let preview = experiments::institutions::preview("harbor-review")?;
    assert_eq!(
        preview["runtime_available"], true,
        "{}",
        preview["runtime_error"]
    );
    let mut run = Run::new(Config::default())?;
    run.world = serde_json::from_value(preview["world"].clone())?;
    run.experiment = Some(experiments::Experiment::HarborReview);
    assert!(!run.world.harbor_onto);
    assert!(run.world.harbor_review);
    assert!(run.world.hosting.is_none());
    let definition = run.world.ontology.as_ref().unwrap().definition.clone();
    assert_eq!(definition.actor, experiments::harbor_onto::ACTOR);
    assert_eq!(
        preview["institution"]["source"]["harbor-works.onto"],
        json!(experiments::institutions::definition("harbor")?.source["harbor-works.onto"])
    );
    assert_eq!(
        run.world.ontology.as_ref().unwrap().snapshot["instances"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["type"] == "Firm")
            .count(),
        14
    );
    assert_eq!(story_executor::generation_status(&run)["can_expand"], false);
    for action in [
        tools::call("shell.exec", json!({"command":"echo forbidden"})),
        tools::call(
            "http.request",
            json!({"method":"POST","url":"https://institutions.local/commands","body":"{}"}),
        ),
        tools::call(
            "file.write",
            json!({"path":"evidence","content":"pretend approved"}),
        ),
        tools::call(
            "mail.send",
            json!({"to":"customer@workspace.local","subject":"Approve","body":"Please approve"}),
        ),
        tools::call("wait", json!({"seconds":86400})),
        tools::call("onto.expand", json!({"module":"payments"})),
    ] {
        let before = run.world.clone();
        assert!(matches!(
            construction::direct(&run, &action)?.resolution,
            construction::Resolution::Denied
        ));
        assert!(run.world.apply(&action).is_err());
        assert_eq!(run.world, before);
    }
    let command = |id: &str, story: &str, extra: Value| {
        let mut inputs = json!({"command_id":id,"story":story});
        inputs
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        tools::call("onto.execute", inputs)
    };
    let delivery = |id: &str, service: &str| {
        command(
            id,
            "Business delivers Agreement with Service",
            json!({"object":"Agreement:fixture-authorized","participants_json":json!({"Service":service}).to_string()}),
        )
    };
    let refused = run
        .world
        .apply(&delivery("wrong-capability", "Service:fixture-delivery"))?;
    assert_eq!(refused["committed"], false, "{refused}");
    assert_eq!(refused["changes"], json!([]));
    let accepted = run
        .world
        .apply(&delivery("delivery", "Service:fixture-directory"))?;
    assert_eq!(accepted["committed"], true, "{accepted}");
    assert!(
        accepted["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["id"] == "Agreement:fixture-authorized"
                && v["after"]["fields"]["status"] == "delivered")
    );
    assert_eq!(
        run.world
            .apply(&delivery("duplicate-delivery", "Service:fixture-directory"))?["committed"],
        false
    );
    let snapshot = run.world.ontology.as_ref().unwrap().snapshot.clone();
    let replay = run
        .world
        .apply(&delivery("delivery", "Service:fixture-directory"))?;
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["changes"], json!([]));
    assert_eq!(snapshot, run.world.ontology.as_ref().unwrap().snapshot);
    // Buyer and verifier Stories cannot be executed as the fixed seller.
    for (id, story, object) in [
        (
            "buyer-payment",
            "Business pays Agreement",
            "Agreement:fixture-accepted",
        ),
        (
            "verifier",
            "Verifier verifies Service",
            "Service:fixture-directory",
        ),
    ] {
        let snapshot = run.world.ontology.as_ref().unwrap().snapshot.clone();
        let result = run
            .world
            .apply(&command(id, story, json!({"object":object})));
        assert!(result.is_err() || result.as_ref().unwrap()["committed"] == false);
        assert_eq!(snapshot, run.world.ontology.as_ref().unwrap().snapshot);
    }
    let proposal = |id: &str, buyer: &str, price: i64, units: i64| {
        command(
            id,
            "Business proposes Agreement to Firm",
            json!({"participants_json":json!({"Firm":buyer}).to_string(),"fields_json":json!({"name":id,"capability":"directory","price":price,"units":units,"deadline":"2026-01-22T00:00:00","status":"paid"}).to_string()}),
        )
    };
    for action in [
        proposal("zero-units", "Firm:market", 10000, 0),
        proposal("self-dealing", "Firm:harbor", 10000, 1),
        proposal("buyer-funds", "Firm:quay", 30000, 1),
        proposal("seller-capacity", "Firm:market", 10000, 10),
    ] {
        let result = run.world.apply(&action)?;
        assert_eq!(result["committed"], false, "{result}");
        assert_eq!(result["changes"], json!([]));
    }
    let proposed = run
        .world
        .apply(&proposal("valid-proposal", "Firm:market", 10000, 1))?;
    assert_eq!(proposed["committed"], true, "{proposed}");
    let state = run.world.ontology.as_ref().unwrap();
    let facts = state.snapshot["instances"].as_array().unwrap();
    assert_eq!(
        facts
            .iter()
            .find(|v| v["fields"]["name"] == "valid-proposal")
            .unwrap()["fields"]["status"],
        "proposed"
    );
    let calendar = facts.iter().find(|v| v["id"] == "Calendar:month").unwrap();
    assert_eq!(
        calendar["fields"]["cashTotal"],
        calendar["fields"]["openingTotal"]
    );
    assert_eq!(state.definition, definition);
    assert_eq!(run.world.tools.as_ref().unwrap().elapsed_seconds, 0);
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("harbor-review.sqlite");
    let mut store = Store::open(path.to_str().unwrap())?;
    store.commit(
        &mut run,
        "outcome",
        "Direct Harbor Story evidence",
        json!({}),
    )?;
    drop(store);
    let restored = Store::open(path.to_str().unwrap())?.load(&run.id)?;
    assert_eq!(
        restored.experiment,
        Some(experiments::Experiment::HarborReview)
    );
    assert_eq!(restored.checkpoint_hash(), run.checkpoint_hash());
    assert_eq!(restored.world.ontology, run.world.ontology);
    assert_eq!(restored.calls, 0);
    Ok(())
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
        json!({"fields_json":"{"}),
        json!({"fields_json":"null"}),
        json!({"fields_json":{}}),
        json!({"participants_json":"{\"Policy\":4}"}),
    ] {
        assert!(
            tools::validate_action(&execute("claims", "test", "Operator expands Policy", extra))
                .is_err()
        );
    }
    for value in [json!(null), json!(""), json!("  \n"), json!("{}")] {
        tools::validate_action(&execute(
            "repair",
            "test",
            "Operator approves RepairRequest",
            json!({"fields_json":value,"participants_json":value}),
        ))
        .unwrap();
    }
}

#[test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-onto"]
fn inspection_limit_requires_new_story_evidence_and_survives_reopen() -> Result<()> {
    let mut run = setup()?;
    let inspect = tools::call("onto.inspect", json!({"module":"repair","view":"facts"}));
    let apply = |run: &mut Run, action: &frontier::Action| -> Result<Value> {
        let result = run.world.apply(action)?;
        run.world
            .transcript
            .push(json!({"action":action,"result":result}));
        Ok(result)
    };
    for _ in 0..3 {
        apply(&mut run, &inspect)?;
    }
    assert_eq!(
        story_executor::progress(&run)["must_execute_or_finish"],
        true
    );
    assert_eq!(
        story_executor::available_definitions(&run)
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["onto.execute", "finish"]
    );
    for action in [
        &inspect,
        &tools::call("onto.expand", json!({"module":"repair"})),
    ] {
        assert!(construction::direct(&run, action).is_err());
        let before = run.world.clone();
        assert!(run.world.apply(action).is_err());
        assert_eq!(run.world, before);
    }
    // A refusal is useful evidence too; it must restore the ability to inspect.
    let complete = execute(
        "repair",
        "needs-approval",
        "Operator completes RepairRequest",
        json!({"object":"RepairRequest:intake","fields_json":"","participants_json":"  "}),
    );
    assert_eq!(apply(&mut run, &complete)?["committed"], false);
    assert_eq!(
        story_executor::progress(&run)["must_execute_or_finish"],
        false
    );
    for _ in 0..3 {
        apply(&mut run, &inspect)?;
    }
    let mut identical = complete.clone();
    identical.inputs.as_mut().unwrap()["fields_json"] = json!("{}");
    identical.inputs.as_mut().unwrap()["participants_json"] = json!("{}");
    assert_eq!(apply(&mut run, &identical)?["replayed"], true);
    assert_eq!(
        story_executor::progress(&run)["must_execute_or_finish"],
        true
    );
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("read-limit.sqlite");
    let mut store = Store::open(path.to_str().unwrap())?;
    store.commit(&mut run, "outcome", "Read limit", json!({}))?;
    drop(store);
    let saved = Store::open(path.to_str().unwrap())?.load(&run.id)?;
    assert_eq!(
        story_executor::progress(&saved),
        story_executor::progress(&run)
    );
    assert!(
        saved.world.ontology.as_ref().unwrap().modules["repair"]
            .receipts
            .len()
            == 1
    );
    Ok(())
}

#[test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-onto"]
fn authored_review_runs_without_generation_and_enforces_its_rules() -> Result<()> {
    let mut run = setup()?;
    let foundation = run.world.ontology.as_ref().unwrap().snapshot.clone();
    let source = run.world.ontology.as_ref().unwrap().modules["repair"]
        .definition
        .clone();
    assert_eq!(
        run.world.ontology.as_ref().unwrap().modules["repair"]
            .stories
            .as_array()
            .unwrap()
            .len(),
        5
    );
    assert_eq!(run.calls, 0);
    assert_eq!(run.expansions, 0);
    assert_eq!(source.agent_state.as_ref().unwrap().fields.len(), 0);
    assert!(run.world.observation()["agent_state"]["foundation"].is_object());

    let complete = |id| {
        execute(
            "repair",
            id,
            "Operator completes RepairRequest",
            json!({"object":"RepairRequest:intake"}),
        )
    };
    let refused = run.world.apply(&complete("premature"))?;
    assert_eq!(refused["committed"], false, "{refused}");
    assert_eq!(refused["changes"], json!([]));
    let approved = run.world.apply(&execute(
        "repair",
        "approve",
        "Operator approves RepairRequest",
        json!({"object":"RepairRequest:intake"}),
    ))?;
    assert_eq!(approved["committed"], true, "{approved}");
    assert_eq!(run.world.apply(&complete("complete"))?["committed"], true);
    assert_eq!(run.world.apply(&complete("twice"))?["committed"], false);
    let state = run.world.ontology.clone();
    let replay = run.world.apply(&complete("complete"))?;
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["changes"], json!([]));
    assert_eq!(run.world.ontology, state);

    let create = |id: &str, cost| {
        execute(
            "repair",
            id,
            "Operator creates RepairRequest for RepairPolicy",
            json!({
                "participants_json":json!({"RepairPolicy":"RepairPolicy:standard"}).to_string(),
                "fields_json":json!({"name":id,"cost":cost,"approved":true,"completed":true}).to_string()
            }),
        )
    };
    for (id, cost) in [("zero", 0), ("over-limit", 26)] {
        let refused = run.world.apply(&create(id, cost))?;
        assert_eq!(refused["committed"], false, "{refused}");
        assert_eq!(refused["changes"], json!([]));
    }
    let accepted = run.world.apply(&create("within-limit", 20))?;
    assert_eq!(accepted["committed"], true, "{accepted}");
    let created = &accepted["changes"][0]["after"]["fields"];
    assert_eq!(created["approved"], false);
    assert_eq!(created["completed"], false);
    let policy = |id, story| {
        execute(
            "repair",
            id,
            story,
            json!({"object":"RepairPolicy:standard"}),
        )
    };
    assert_eq!(
        run.world
            .apply(&policy("expand", "Operator expands RepairPolicy"))?["committed"],
        true
    );
    assert_eq!(
        run.world.apply(&create("higher-limit", 50))?["committed"],
        true
    );
    // Restoring a lower limit would invalidate an already committed request.
    let restore = run
        .world
        .apply(&policy("restore", "Operator restores RepairPolicy"))?;
    assert_eq!(restore["committed"], false, "{restore}");
    assert_eq!(restore["changes"], json!([]));
    assert_eq!(run.world.ontology.as_ref().unwrap().snapshot, foundation);
    assert_eq!(
        run.world.ontology.as_ref().unwrap().modules["repair"].definition,
        source
    );

    // A full generation budget must never hide existing executable Stories.
    run.config.expansion_limit = 0;
    let observation = frontier::memory::observation(&run);
    assert_eq!(observation["generation_status"]["can_expand"], false);
    let names: Vec<_> = observation["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["onto.inspect", "onto.execute", "finish"]);
    assert_eq!(observation["available_modules"], json!(["repair"]));
    assert!(
        construction::direct(&run, &tools::call("onto.expand", json!({"module":"new"})))
            .unwrap_err()
            .to_string()
            .contains("Expansion limit")
    );
    run.config.expansion_limit = 1;
    run.world
        .ontology
        .as_mut()
        .unwrap()
        .generation
        .as_mut()
        .unwrap()
        .max_modules = 1;
    assert_eq!(story_executor::generation_status(&run)["can_expand"], false);
    assert_eq!(
        story_executor::generation_status(&run)["remaining_module_slots"],
        0
    );
    Ok(())
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
