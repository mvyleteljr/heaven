use super::*;

#[tokio::test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-jit"]
async fn jit_expansion_demo_seed_generates_retries_and_commits_a_ticket() -> Result<()> {
    let preview = experiments::institutions::preview("expansion-demo")?;
    assert_eq!(preview["runtime_available"], true);
    assert_eq!(preview["generation"]["max_modules"], 1);
    assert_eq!(preview["config"]["dollar_limit"], 0.5);
    let mut config: Config = serde_json::from_value(preview["config"].clone())?;
    config.actor_system_prompt = Some(preview["system"].as_str().unwrap().into());
    for settings in [&mut config.actor_settings, &mut config.generator_settings] {
        settings.as_mut().unwrap().endpoint = "mock".into();
    }
    let (mut app, mut run) = app(config)?;
    run.world = serde_json::from_value(preview["world"].clone())?;
    assert!(run.world.ontology.as_ref().unwrap().modules.is_empty());
    run.experiment = Some(experiments::Experiment::Institutions);
    app.store
        .lock()
        .unwrap()
        .commit(&mut run, "seed", "Expansion demo", json!({}))?;
    let boundary = "https://institutions.local/modules/repair-intake";
    let actions = vec![
        json!({"kind":"http.request","inputs":{"method":"GET","url":boundary}}),
        json!({"kind":"http.request","inputs":{"method":"GET","url":format!("{boundary}/model")}}),
        json!({"kind":"http.request","inputs":{"method":"GET","url":format!("{boundary}/facts")}}),
        json!({"kind":"http.request","inputs":{"method":"POST","url":format!("{boundary}/commands"),"body":json!({"command_id":"intake-1","story":"Operator creates RepairRequest for IntakeDesk","participants":{"IntakeDesk":"IntakeDesk:main"},"fields":{"description":"Cracked laptop screen"}}).to_string()}}),
        json!({"kind":"http.request","inputs":{"method":"GET","url":format!("{boundary}/facts")}}),
        json!({"kind":"finish","inputs":{"message":"Verified the saved repair request."}}),
    ];
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let generated = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (c, g) = (count.clone(), generated.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    app.inference_url = format!("http://{}/completion", listener.local_addr()?);
    let router = Router::new().route("/completion", post(move |Json(request): Json<Value>| {
        let (c, g, actions) = (c.clone(), g.clone(), actions.clone());
        async move {
            let output = if request["messages"][0]["content"] == frontier::institutions::GENERATOR {
                g.fetch_add(1, Ordering::SeqCst);
                json!({"source":include_str!("../examples/jit/test-repair-intake.onto"),"agent_state":{"fields":[]},"error":null})
            } else {
                assert!(request.to_string().contains("Your first action must be http.request"));
                typed_actor_output(&request, &actions[c.fetch_add(1, Ordering::SeqCst)])
            };
            Json(json!({"usage":{"cost":0.0001},"choices":[{"finish_reason":"stop","message":{"content":output.to_string()}}]}))
        }
    }));
    let _mock = Mock(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    for _ in 0..6 {
        step(&app, &run.id).await?;
    }
    let store = app.store.lock().unwrap();
    let saved = store.load(&run.id)?;
    assert_eq!(saved.status, "finished");
    assert_eq!(generated.load(Ordering::SeqCst), 1);
    assert_eq!(saved.expansions, 1);
    let frames = store.frames(&run.id)?;
    let blocked = frames
        .iter()
        .find(|f| f["event"]["kind"] == "action_blocked")
        .unwrap();
    let ready = frames
        .iter()
        .find(|f| f["event"]["kind"] == "action_ready")
        .unwrap();
    let completed = frames
        .iter()
        .find(|f| {
            f["event"]["kind"] == "outcome"
                && f["event"]["evidence"]["action_id"] == blocked["event"]["evidence"]["action_id"]
        })
        .unwrap();
    assert!(blocked["world"]["ontology"]["modules"]["repair-intake"].is_null());
    assert!(
        ready["world"]["ontology"]["modules"]["repair-intake"]["definition"]["source"]
            .to_string()
            .contains("object RepairRequest")
    );
    assert_eq!(
        blocked["event"]["evidence"]["action"],
        completed["event"]["evidence"]["action"]
    );
    assert_eq!(completed["event"]["evidence"]["result"]["status"], 200);
    let receipt: Value = serde_json::from_str(
        saved.world.transcript[3]["result"]["body"]
            .as_str()
            .unwrap(),
    )?;
    assert_eq!(receipt["committed"], true);
    let facts: Value = serde_json::from_str(
        saved.world.transcript[4]["result"]["body"]
            .as_str()
            .unwrap(),
    )?;
    assert!(facts.to_string().contains("Cracked laptop screen"));
    assert!(facts.to_string().contains("pending"));
    assert_eq!(
        blocked["world"]["ontology"]["agent_state"]["values"]["name"],
        "Ada"
    );
    let module = &saved.world.ontology.as_ref().unwrap().modules["repair-intake"];
    let empty = json!({"shape":{"fields":[]},"values":{}});
    assert_eq!(
        ready["world"]["ontology"]["modules"]["repair-intake"]["agent_state"],
        empty
    );
    assert_eq!(module.agent_state.as_ref().unwrap(), &empty);
    assert_eq!(facts["agent_state"], empty);
    assert_eq!(
        saved.world.observation()["agent_state"]["module:repair-intake"],
        empty
    );
    // The owned desk changes in world state, never in the agent projection.
    for (snapshot, expected) in [
        (
            &ready["world"]["ontology"]["modules"]["repair-intake"]["snapshot"],
            "ready",
        ),
        (&module.snapshot, "registered"),
    ] {
        let desk = snapshot["instances"]
            .as_array()
            .unwrap()
            .iter()
            .find(|record| record["id"] == "IntakeDesk:main")
            .unwrap();
        assert_eq!(desk["fields"]["status"], expected);
    }
    // Reloaded runs preserve the declaration and live values; prior frames retain the old shape.
    let reopened: Run = serde_json::from_value(serde_json::to_value(&saved)?)?;
    assert_eq!(reopened.world, saved.world);
    Ok(())
}

#[tokio::test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-jit"]
async fn jit_story_review_uses_direct_tools_and_existing_compiler_pipeline() -> Result<()> {
    let (mut app, mut run) = app(Config {
        demo: false,
        structured_actions: true,
        actor_settings: Some(frontier::ActorSettings {
            provider: "openrouter".into(),
            temperature: None,
            max_tokens: 4096,
            reasoning_effort: None,
            endpoint: "mock".into(),
        }),
        dollar_limit: 2.0,
        ..Config::default()
    })?;
    let preview = experiments::institutions::preview("story-review")?;
    run.world = serde_json::from_value(preview["world"].clone())?;
    run.experiment = Some(experiments::Experiment::OntoReview);
    run.config.actor_system_prompt = Some(frontier::story_executor::SYSTEM.into());
    app.store
        .lock()
        .unwrap()
        .commit(&mut run, "seed", "Story review", json!({}))?;
    let actions = vec![
        json!({"kind":"onto.expand","inputs":{"module":"claims"}}),
        json!({"kind":"onto.inspect","inputs":{"module":"claims","view":"model"}}),
        json!({"kind":"onto.execute","inputs":{"module":"claims","command_id":"review-policy","story":"Operator expands Policy","object":"Policy:main"}}),
        json!({"kind":"onto.expand","inputs":{"module":"claims"}}),
        json!({"kind":"finish","inputs":{"message":"Policy changed from 10 to 50; receipt review-policy."}}),
    ];
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let generated = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (c, g) = (count.clone(), generated.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    app.inference_url = format!("http://{}/completion", listener.local_addr()?);
    let router = Router::new().route("/completion", post(move |Json(request): Json<Value>| {
        let (c,g,actions) = (c.clone(),g.clone(),actions.clone());
        async move {
            let output = if request["messages"][0]["content"] == frontier::institutions::GENERATOR {
                g.fetch_add(1,Ordering::SeqCst);
                json!({"source":include_str!("../examples/jit/test-capability.onto"),"agent_state":{"fields":[]},"error":null})
            } else {
                assert!(!request["messages"][0]["content"].as_str().unwrap().contains("http.request"));
                let schema = request["response_format"].to_string();
                assert!(!schema.contains("shell.exec") && !schema.contains("http.request"));
                typed_actor_output(&request, &actions[c.fetch_add(1,Ordering::SeqCst)])
            };
            Json(json!({"usage":{"cost":0.0001},"choices":[{"finish_reason":"stop","message":{"content":output.to_string()}}]}))
        }
    }));
    let _mock = Mock(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    for _ in 0..5 {
        step(&app, &run.id).await?;
    }
    let store = app.store.lock().unwrap();
    let saved = store.load(&run.id)?;
    assert_eq!(saved.status, "finished");
    assert_eq!(saved.calls, 6);
    let frames = store.frames(&run.id)?;
    let blocked = frames
        .iter()
        .find(|f| f["event"]["kind"] == "action_blocked")
        .unwrap();
    let ready = frames
        .iter()
        .find(|f| f["event"]["kind"] == "action_ready")
        .unwrap();
    let completed = frames
        .iter()
        .find(|f| {
            f["event"]["kind"] == "outcome"
                && f["event"]["evidence"]["action_id"] == blocked["event"]["evidence"]["action_id"]
        })
        .unwrap();
    assert!(blocked["world"]["ontology"]["modules"]["claims"].is_null());
    assert!(ready["world"]["ontology"]["modules"]["claims"].is_object());
    assert_eq!(
        blocked["event"]["evidence"]["action"],
        ready["event"]["evidence"]["action"]
    );
    assert_eq!(
        blocked["event"]["evidence"]["action"],
        completed["event"]["evidence"]["action"]
    );
    assert_eq!(completed["event"]["evidence"]["result"]["module"], "claims");
    assert_eq!(generated.load(Ordering::SeqCst), 1);
    assert_eq!(saved.expansions, 1);
    assert_eq!(saved.world.transcript[2]["result"]["committed"], true);
    assert_eq!(
        saved.world.transcript[2]["result"]["changes"][0]["after"]["fields"]["limit"],
        50
    );
    assert_eq!(
        store
            .events(&run.id)?
            .iter()
            .filter(|e| e.kind == "published")
            .count(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn invalid_tool_input_returns_actionable_feedback_without_generation() -> Result<()> {
    let (mut app, mut run) = app(Config {
        demo: false,
        ..Config::default()
    })?;
    run.world = experiments::bellwether::seed();
    run.experiment = Some(experiments::Experiment::Bellwether);
    app.store
        .lock()
        .unwrap()
        .commit(&mut run, "seed", "business", json!({}))?;
    let _mock = provider(&mut app,StatusCode::OK,json!({"usage":{"cost":0.001},"choices":[{"message":{"content":json!({"kind":"http.request","inputs":{"buyer":"quay"}}).to_string()}}]}).to_string()).await?;
    step(&app, &run.id).await?;
    let store = app.store.lock().unwrap();
    let saved = store.load(&run.id)?;
    assert_eq!(saved.actions, 1);
    assert_eq!(saved.calls, 1);
    assert_eq!(saved.expansions, 0);
    assert_eq!(saved.world.observation()["elapsed_seconds"], 60);
    let error = saved.world.transcript[0]["result"]["error"]
        .as_str()
        .unwrap();
    assert!(
        error.contains("Invalid tool input") && error.contains("buyer") && error.contains("url")
    );
    assert!(
        !store
            .events(&run.id)?
            .iter()
            .any(|e| e.kind == "construction_rejected" || e.kind == "construction_request")
    );
    Ok(())
}

#[tokio::test]
async fn long_trajectory_compacts_constructs_recalls_and_reopens() -> Result<()> {
    let (mut app, mut run) = app(Config {
        demo: false,
        structured_actions: true,
        actor_settings: Some(frontier::ActorSettings {
            provider: "openrouter".into(),
            temperature: None,
            max_tokens: 4096,
            reasoning_effort: Some("low".into()),
            endpoint: "mock".into(),
        }),
        action_limit: 300,
        call_limit: 400,
        dollar_limit: 2.0,
        context_limit_bytes: 64_000,
        memory: Some(memory::Policy::default()),
        ..Config::default()
    })?;
    run.world = experiments::bellwether::seed();
    run.experiment = Some(experiments::Experiment::Bellwether);
    memory::configure(&mut run)?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("stress.sqlite");
    run.seq = 0;
    app.store = Arc::new(Mutex::new(Store::open(path.to_str().unwrap())?));
    app.store
        .lock()
        .unwrap()
        .commit(&mut run, "seed", "stress", json!({}))?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    app.inference_url = format!("http://{}/completion", listener.local_addr()?);
    let actor_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let calls = actor_calls.clone();
    let router = Router::new().route("/completion",post(move |Json(request): Json<Value>| {
        let calls = calls.clone();
        async move {
            assert!(serde_json::to_vec(&request).unwrap().len() <= 64_000);
            let system = request["messages"][0]["content"].as_str().unwrap();
            let input: Value = serde_json::from_str(request["messages"][1]["content"].as_str().unwrap()).unwrap();
            let output = if system == memory::SYSTEM {
                assert_eq!(request["max_tokens"],4096);
                json!({"items":[{"kind":"fact","text":"Observed account and public profile; retrieve original evidence before acting.","sources":[input["history"][0]["index"]]}]})
            } else if system == construction::GENERATOR {
                json!({"version":1,"request_id":input["request_id"],"basis":input["basis"],"result":{"status":"proposed","resources":[{"target":input["directive"]["target"],"operation":"create","initial_state":{"title":"Quay Hall","text":"Local venue."},"interface":input["interface"],"functionality":input["directive"]["requirements"]["functionality"],"dependencies":input["directive"]["requirements"]["dependencies"]}]}})
            } else {
                let n = calls.fetch_add(1,Ordering::SeqCst);
                if n == 0 { json!({"kind":"http.request","inputs":{"method":"GET","url":"https://bellwether.local/organizations/quay/about"}}) }
                else if n == 50 { json!({"kind":"history.read","inputs":{"index":0}}) }
                else if n == 150 { json!({"kind":"history.search","inputs":{"query":"Quay"}}) }
                else if n >= 220 { json!({"kind":"wait","inputs":{"seconds":604800}}) }
                else { json!({"kind":"http.request","inputs":{"method":"GET","url":"https://bellwether.local/account"}}) }
            };
            let output = if output.get("kind").is_some() {
                assert_eq!(request["response_format"]["json_schema"]["strict"],true);
                typed_actor_output(&request, &output)
            } else { output };
            Json(json!({"usage":{"cost":0.001},"choices":[{"finish_reason":"stop","message":{"content":output.to_string()}}]}))
        }
    }));
    let _mock = Mock(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    for n in 0..300 {
        step(&app, &run.id).await?;
        if n == 110 {
            let hash = app.store.lock().unwrap().load(&run.id)?.checkpoint_hash();
            app.store = Arc::new(Mutex::new(Store::open(path.to_str().unwrap())?));
            assert_eq!(
                app.store.lock().unwrap().load(&run.id)?.checkpoint_hash(),
                hash
            );
        }
        if app.store.lock().unwrap().load(&run.id)?.world.finished {
            break;
        }
    }
    let store = app.store.lock().unwrap();
    let saved = store.load(&run.id)?;
    assert!(saved.world.finished);
    assert!(saved.actions > 220);
    assert_eq!(saved.world.transcript.len(), saved.actions as usize);
    assert_eq!(saved.expansions, 1);
    assert!(saved.memory.as_ref().unwrap().revisions >= 15);
    // Repeated floating-point reservations can leave sub-cent rounding dust.
    assert!(saved.reserved.abs() < 1e-12);
    assert_eq!(
        saved.calls,
        saved.actions + saved.expansions + saved.memory.as_ref().unwrap().revisions
    );
    assert!((saved.charged - saved.calls as f64 * 0.001).abs() < 1e-9);
    assert!(
        saved.world.transcript[50]["result"]["content"]
            .as_str()
            .unwrap()
            .contains("Quay Hall")
    );
    assert!(
        !saved.world.transcript[150]["result"]["matches"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        saved.world.tools.as_ref().unwrap().elapsed_seconds,
        frontier::business::HORIZON
    );
    assert_eq!(serde_json::to_value(&saved.world.business)?["checkins"], 4);
    assert!(
        !store
            .events(&run.id)?
            .iter()
            .any(|e| e.kind == "construction_rejected" || e.kind == "error")
    );
    Ok(())
}

#[tokio::test]
async fn invalid_memory_stops_without_losing_history_and_is_charged() -> Result<()> {
    let (mut app, mut run) = app(Config {
        demo: false,
        context_limit_bytes: 64_000,
        memory: Some(memory::Policy::default()),
        ..Config::default()
    })?;
    run.world = experiments::bellwether::seed();
    memory::configure(&mut run)?;
    run.world.transcript = (0..25).map(|i| json!({"result":i})).collect();
    app.store
        .lock()
        .unwrap()
        .commit(&mut run, "outcome", "history", json!({}))?;
    let hash = serde_json::to_value(&run.memory)?;
    let _mock = provider(&mut app,StatusCode::OK,json!({"usage":{"cost":0.002},"choices":[{"message":{"content":"{\"items\":[{\"kind\":\"fact\",\"text\":\"bad citation\",\"sources\":[99]}]}"}}]}).to_string()).await?;
    assert!(
        prepare_memory(&app, &run.id)
            .await
            .unwrap_err()
            .to_string()
            .contains("not supplied")
    );
    let saved = app.store.lock().unwrap().load(&run.id)?;
    assert_eq!(saved.world, run.world);
    assert_eq!(serde_json::to_value(&saved.memory)?, hash);
    assert_eq!(saved.calls, 1);
    assert_eq!(saved.charged, 0.002);
    assert_eq!(saved.actions, 0);
    Ok(())
}

#[test]
fn structured_action_transport_is_strict_and_preserves_tool_inputs() -> Result<()> {
    let mut run = Run::new(Config::default())?;
    run.config.structured_actions = true;
    run.world = experiments::bellwether::seed();
    let action = parse_actor_action(
        &run,
        r#"{"kind":"file.write","inputs_json":"{\"path\":\"a\",\"content\":\"hello\\nworld\"}"}"#,
    )?;
    assert_eq!(action.inputs.unwrap()["content"], "hello\nworld");
    for bad in [
        r#"{"kind":"mail.list","inputs_json":"{}"} {"kind":"mail.list","inputs_json":"{}"}"#,
        r#"{"kind":"mail.list","inputs_json":"[]"}"#,
        r#"{"kind":"mail.list","inputs_json":"{","extra":true}"#,
    ] {
        assert!(parse_actor_action(&run, bad).is_err());
    }
    run.config.structured_actions = false;
    assert!(parse_actor_action(&run, r#"{"kind":"mail.list","inputs":{}}"#).is_ok());
    Ok(())
}

// Build a mock reply from the actual outgoing schema, including nullable inputs.
fn typed_actor_output(request: &Value, action: &Value) -> Value {
    let branches =
        request["response_format"]["json_schema"]["schema"]["properties"]["action"]["anyOf"]
            .as_array()
            .unwrap();
    let branch = branches
        .iter()
        .find(|branch| branch["properties"]["kind"]["enum"][0] == action["kind"])
        .unwrap();
    let mut inputs = action["inputs"].as_object().unwrap().clone();
    for key in branch["properties"]["inputs"]["properties"]
        .as_object()
        .unwrap()
        .keys()
    {
        inputs.entry(key.clone()).or_insert(Value::Null);
    }
    json!({"action":{"kind":action["kind"],"inputs":inputs}})
}

#[test]
fn strict_actor_schema_requires_actual_tool_arguments() -> Result<()> {
    let mut run = Run::new(Config::default())?;
    run.world = experiments::bellwether::seed();
    run.config.structured_actions = true;
    run.config.memory = Some(memory::Policy::default());
    memory::configure(&mut run)?;
    let request = inference_request(&run, "actor", tools::SYSTEM, &json!({}))?;
    let schema = &request["response_format"]["json_schema"]["schema"];
    assert_eq!(
        request["response_format"]["json_schema"]["name"],
        "actor_action_v2"
    );
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["required"], json!(["action"]));
    assert!(schema.get("anyOf").is_none());
    let branches = schema["properties"]["action"]["anyOf"].as_array().unwrap();
    assert_eq!(branches.len(), 12);
    for branch in branches {
        let inputs = &branch["properties"]["inputs"];
        let fields = inputs["properties"].as_object().unwrap();
        assert_eq!(inputs["required"].as_array().unwrap().len(), fields.len());
        assert_eq!(inputs["additionalProperties"], false);
        for field in fields.keys() {
            assert!(
                inputs["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(field))
            );
        }
        if branch["properties"]["kind"]["enum"][0] == "finish" {
            assert_eq!(inputs["required"], json!(["message"]));
            assert_eq!(inputs["properties"]["message"]["type"], "string");
            assert_eq!(inputs["properties"]["message"]["minLength"], 1);
        }
        if branch["properties"]["kind"]["enum"][0] == "http.request" {
            for field in ["method", "url"] {
                assert_eq!(inputs["properties"][field]["type"], "string");
                assert!(inputs["properties"][field].get("anyOf").is_none());
            }
            assert_eq!(inputs["properties"]["headers"]["anyOf"][0]["type"], "array");
            assert_eq!(
                inputs["properties"]["headers"]["anyOf"][0]["items"]["additionalProperties"],
                false
            );
        }
    }
    for bad in [
        json!({"action":{"kind":"finish","inputs":{}}}),
        json!({"action":{"kind":"finish","inputs":{"message":null}}}),
        json!({"action":{"kind":"finish","inputs":{"message":" "}}}),
        json!({"action":{"kind":"http.request","inputs":{}}}),
        json!({"action":{"kind":"http.request","inputs":{"method":null,"url":null,"headers":null,"body":null}}}),
        json!({"action":{"kind":"finish","inputs":{"message":"Done","extra":null}}}),
        json!({"action":{"kind":"unknown","inputs":{}}}),
    ] {
        assert!(
            parse_actor_action(&run, &bad.to_string()).is_err(),
            "accepted {bad}"
        );
    }
    Ok(())
}

#[test]
fn typed_actor_transport_preserves_all_tools_and_nullable_optionals() -> Result<()> {
    let mut run = Run::new(Config::default())?;
    run.world = experiments::bellwether::seed();
    run.config.structured_actions = true;
    run.config.memory = Some(memory::Policy::default());
    memory::configure(&mut run)?;
    let request = inference_request(&run, "actor", tools::SYSTEM, &json!({}))?;
    for (kind, inputs) in [
        ("search", json!({"query":"Harbor"})),
        (
            "http.request",
            json!({"method":"GET","url":"https://institutions.local/facts"}),
        ),
        ("shell.exec", json!({"command":"pwd"})),
        ("file.read", json!({"path":"a"})),
        ("file.write", json!({"path":"a","content":"hello\nworld"})),
        (
            "mail.send",
            json!({"to":"a@example.test","subject":"Update","body":"Confirmed"}),
        ),
        ("mail.list", json!({})),
        ("mail.read", json!({"message_id":"m1"})),
        ("wait", json!({"seconds":0})),
        ("finish", json!({"message":"The task is complete."})),
        ("history.read", json!({"index":0})),
        ("history.search", json!({"query":"Harbor"})),
        ("file.read", json!({"path":"a","offset":4,"limit":32})),
        ("mail.list", json!({"cursor":"next"})),
        (
            "mail.send",
            json!({"to":"a@example.test","subject":"Update","body":"Confirmed","reply_to":"m1"}),
        ),
        ("history.read", json!({"index":0,"offset":1,"limit":256})),
        ("history.search", json!({"query":"Harbor","cursor":4})),
    ] {
        let output = typed_actor_output(&request, &json!({"kind":kind,"inputs":inputs}));
        let action = parse_actor_action(&run, &output.to_string())?;
        assert_eq!(action.kind, kind);
        assert_eq!(action.inputs, Some(inputs));
        tools::validate_action(&action)?;
    }
    let http = json!({"action":{"kind":"http.request","inputs":{"method":"POST","url":"https://institutions.local/commands","body":"{\"story\":\"example\"}","headers":[{"name":"Content-Type","value":"application/json"}]}}});
    let action = parse_actor_action(&run, &http.to_string())?;
    assert_eq!(
        action.inputs.as_ref().unwrap()["headers"],
        json!({"Content-Type":"application/json"})
    );
    assert_eq!(
        action.inputs.as_ref().unwrap()["body"],
        "{\"story\":\"example\"}"
    );
    let mut duplicate = http;
    duplicate["action"]["inputs"]["headers"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"content-type","value":"text/plain"}));
    assert!(parse_actor_action(&run, &duplicate.to_string()).is_err());
    Ok(())
}

#[tokio::test]
async fn typed_finish_completes_and_invalid_provider_arguments_cannot_consume_actions() -> Result<()>
{
    for (valid, output) in [
        (
            true,
            json!({"action":{"kind":"finish","inputs":{"message":"The confirmed result is recorded."}}}),
        ),
        (false, json!({"action":{"kind":"finish","inputs":{}}})),
        (false, json!({"action":{"kind":"http.request","inputs":{}}})),
        (false, json!({"kind":"finish","inputs_json":"{}"})),
        (false, json!({"kind":"http.request","inputs_json":"{}"})),
    ] {
        let (mut app, mut run) = app(Config {
            demo: false,
            structured_actions: true,
            actor_settings: Some(frontier::ActorSettings {
                provider: "openrouter".into(),
                temperature: None,
                max_tokens: 4096,
                reasoning_effort: None,
                endpoint: "mock".into(),
            }),
            ..Config::default()
        })?;
        run.world = experiments::bellwether::seed();
        app.store
            .lock()
            .unwrap()
            .commit(&mut run, "seed", "tools", json!({}))?;
        let _mock = provider(&mut app, StatusCode::OK, json!({"usage":{"cost":0.001},"choices":[{"finish_reason":"stop","message":{"content":output.to_string()}}]}).to_string()).await?;
        let result = step(&app, &run.id).await;
        assert_eq!(result.is_ok(), valid);
        let store = app.store.lock().unwrap();
        let saved = store.load(&run.id)?;
        assert_eq!(saved.calls, 1);
        assert_eq!(saved.charged, 0.001);
        assert_eq!(saved.actions, u32::from(valid));
        assert_eq!(saved.world.finished, valid);
        assert_eq!(
            store
                .events(&run.id)?
                .iter()
                .filter(|event| event.kind == "actor_rejected")
                .count(),
            usize::from(!valid)
        );
    }
    Ok(())
}

fn app(config: Config) -> Result<(App, Run)> {
    let mut store = Store::open(":memory:")?;
    let mut run = Run::new(config)?;
    store.commit(&mut run, "seed", "test", json!({}))?;
    Ok((
        App {
            store: Arc::new(Mutex::new(store)),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()?,
            key: Some("test-key".into()),
            active: Arc::new(AtomicBool::new(false)),
            inference_url: String::new(),
        },
        run,
    ))
}

#[tokio::test]
async fn repeated_steps_release_worker_and_pause_preserves_terminal_status() -> Result<()> {
    let (app, run) = app(Config {
        action_limit: 2,
        ..Config::default()
    })?;
    let mut headers = HeaderMap::new();
    headers.insert("x-frontier-client", "research-ui".parse()?);
    for expected in 1..=2 {
        let _ = control(
            State(app.clone()),
            Path(run.id.clone()),
            headers.clone(),
            Json(Command {
                command: "step".into(),
            }),
        )
        .await
        .map_err(|e| e.0)?;
        tokio::time::timeout(Duration::from_millis(500), async {
            while app.active.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await?;
        assert_eq!(app.store.lock().unwrap().load(&run.id)?.actions, expected);
    }
    let before = app.store.lock().unwrap().load(&run.id)?;
    assert_eq!(before.status, "exhausted");
    let after = control(
        State(app),
        Path(run.id),
        headers,
        Json(Command {
            command: "pause".into(),
        }),
    )
    .await
    .map_err(|e| e.0)?
    .0;
    assert_eq!(after.status, "exhausted");
    assert_eq!(after.seq, before.seq);
    Ok(())
}

struct Mock(tokio::task::JoinHandle<()>);
impl Drop for Mock {
    fn drop(&mut self) {
        self.0.abort();
    }
}
async fn provider(app: &mut App, status: StatusCode, body: String) -> Result<Mock> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    app.inference_url = format!("http://{}/completion", listener.local_addr()?);
    let router = Router::new().route(
        "/completion",
        post(move |headers: HeaderMap, Json(request): Json<Value>| {
            let body = body.clone();
            async move {
                assert_eq!(headers["authorization"], "Bearer test-key");
                assert_eq!(request["provider"]["allow_fallbacks"], false);
                (status, body)
            }
        }),
    );
    Ok(Mock(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    })))
}

#[tokio::test]
async fn provider_failures_preserve_evidence_and_uncertain_reservations() -> Result<()> {
    for (status, body, cost, expected) in [
        (StatusCode::BAD_GATEWAY, "gateway unavailable".into(), None, "HTTP 502"),
        (StatusCode::OK, json!({"error":{"message":"provider failed"}}).to_string(), None, "no text content"),
        (StatusCode::OK, json!({"usage":{"cost":0.003},"choices":[{"finish_reason":"length","message":{"content":"{\"kind\":"}}]}).to_string(), Some(0.003), "truncated"),
    ] {
        let (mut app, run) = app(Config::default())?;
        let _mock = provider(&mut app, status, body.clone()).await?;
        let error = infer(&app, &run.id, "actor", "system", json!({})).await.unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
        let store = app.store.lock().unwrap();
        let saved = store.load(&run.id)?;
        assert_eq!(saved.calls, 1);
        assert_eq!(saved.charged, cost.unwrap_or(0.0));
        assert_eq!(saved.reserved, if cost.is_some() {0.0} else {0.1});
        let events = store.events(&run.id)?;
        assert_eq!(events.last().unwrap().kind, "inference_failure");
        let response=events.iter().find(|e|e.kind=="response").unwrap();
        if status == StatusCode::BAD_GATEWAY {
            assert_eq!(response.evidence["response"]["raw_body"], body);
        }
    }
    Ok(())
}

#[tokio::test]
async fn generator_transport_failure_stops_after_recording_the_attempt() -> Result<()> {
    let (mut app, mut run) = app(Config {
        demo: false,
        ..Config::default()
    })?;
    run.world = experiments::records::seed();
    run.experiment = Some(experiments::Experiment::Records);
    app.store
        .lock()
        .unwrap()
        .commit(&mut run, "seed", "records", json!({}))?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    app.inference_url = format!("http://{}/completion", listener.local_addr()?);
    let router=Router::new().route("/completion",post(|Json(request):Json<Value>|async move {
        if request["messages"][0]["content"]==construction::GENERATOR {
            (StatusCode::TOO_MANY_REQUESTS,[("retry-after","120")],json!({"error":{"message":"busy"}}).to_string())
        } else {
            (StatusCode::OK,[("retry-after","")],json!({"usage":{"cost":0.001},"choices":[{"message":{"content":json!({"kind":"http.request","inputs":{"method":"GET","url":"https://records.local/ledger"}}).to_string()}}]}).to_string())
        }
    }));
    let _mock = Mock(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap()
    }));
    assert!(
        step(&app, &run.id)
            .await
            .unwrap_err()
            .is::<InferenceFailure>()
    );
    let store = app.store.lock().unwrap();
    let saved = store.load(&run.id)?;
    assert_eq!(saved.calls, 2);
    assert_eq!(saved.actions, 1);
    assert_eq!(saved.world.transcript.len(), 1);
    assert_eq!(saved.reserved, 0.1);
    assert!(!experiments::exists(
        &saved.world,
        "https://records.local/ledger"
    ));
    let events = store.events(&run.id)?;
    assert!(
        events
            .iter()
            .any(|e| e.kind == "response" && e.evidence["retry_after"] == "120")
    );
    assert!(events.iter().any(|e| e.kind == "inference_failure"
        && e.evidence["role"] == "world"
        && e.evidence["phase"] == "provider"));
    let audit = frontier::audit::report(&saved, &events);
    assert_eq!(audit["integrity"]["unresolved_action_attempts"], 0);
    Ok(())
}

#[tokio::test]
async fn archive_handoff_full_history_and_rolling_memory_survive_restart_and_fork() -> Result<()> {
    for rolling in [false, true] {
        let (mut app, mut run) = app(Config {
            demo: false,
            action_limit: 150,
            call_limit: 200,
            context_limit_bytes: 256_000,
            memory: rolling.then_some(memory::Policy {
                recent_entries: 4,
                max_summary_bytes: 8000,
            }),
            ..Config::default()
        })?;
        run.world = experiments::recall::seed();
        run.experiment = Some(experiments::Experiment::Recall);
        memory::configure(&mut run)?;
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("recall.sqlite");
        run.seq = 0;
        app.store = Arc::new(Mutex::new(Store::open(path.to_str().unwrap())?));
        app.store
            .lock()
            .unwrap()
            .commit(&mut run, "seed", "recall", json!({}))?;
        let mut actions = vec![
            json!({"kind":"http.request","inputs":{"method":"GET","url":"https://handoff.local/brief"}}),
        ];
        for n in 1..=48 {
            actions.push(json!({"kind":"http.request","inputs":{"method":"GET","url":format!("https://handoff.local/notes/{n:02}")}}));
        }
        actions.extend([
            json!({"kind":"mail.read","inputs":{"message_id":"correction-1"}}),
            json!({"kind":"wait","inputs":{"seconds":600}}),
            json!({"kind":"file.write","inputs":{"path":"handoff.json","content":json!({"reference_code":"AT-42","previous_reference":"AT-17","destination":"vault/atlas","approved_limit":4,"pending_item":"amber folio","verification_phrase":"copper heron"}).to_string()}}),
            json!({"kind":"finish","inputs":{"message":"Handoff recorded"}}),
        ]);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        app.inference_url = format!("http://{}/completion", listener.local_addr()?);
        let cursor = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let router=Router::new().route("/completion",post(move |Json(request):Json<Value>| {
            let cursor=cursor.clone();let actions=actions.clone();
            async move {
                let input:Value=serde_json::from_str(request["messages"][1]["content"].as_str().unwrap()).unwrap();
                let output=if request["messages"][0]["content"]==memory::SYSTEM {
                    json!({"items":[{"kind":"uncertainty","text":"Consult the exact brief and correction before submitting.","sources":[input["history"][0]["index"]]}]})
                } else {actions[cursor.fetch_add(1,Ordering::SeqCst)].clone()};
                Json(json!({"usage":{"cost":0.001},"choices":[{"message":{"content":output.to_string()}}]}))
            }
        }));
        let _mock = Mock(tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap()
        }));
        for n in 0..53 {
            step(&app, &run.id).await?;
            if n == 30 {
                let hash = app.store.lock().unwrap().load(&run.id)?.checkpoint_hash();
                app.store = Arc::new(Mutex::new(Store::open(path.to_str().unwrap())?));
                let saved = app.store.lock().unwrap().load(&run.id)?;
                assert_eq!(saved.checkpoint_hash(), hash);
                assert_eq!(saved.fork(saved.seq).checkpoint_hash(), hash);
            }
        }
        let store = app.store.lock().unwrap();
        let saved = store.load(&run.id)?;
        let report = frontier::audit::report(&saved, &store.events(&run.id)?);
        assert_eq!(report["outcome"]["task_complete"], true, "{report}");
        assert_eq!(report["integrity"]["events_contiguous"], true);
        assert_eq!(saved.actions, 53);
        assert_eq!(saved.expansions, 0);
        assert_eq!(saved.world.transcript.len(), 53);
        if rolling {
            assert!(saved.memory.unwrap().revisions >= 8);
        }
    }
    Ok(())
}

#[tokio::test]
async fn full_history_limits_are_explicit_and_checked_before_spending() -> Result<()> {
    let input = json!({"history":"observed evidence ".repeat(3000)});
    let (mut app, mut run) = app(Config::default())?;
    let _mock = provider(
        &mut app,
        StatusCode::OK,
        json!({"usage":{"cost":0.002},"choices":[{"message":{"content":"{}"}}]}).to_string(),
    )
    .await?;
    let error = infer(&app, &run.id, "actor", "system", input.clone())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("Context limit reached"));
    assert_eq!(app.store.lock().unwrap().load(&run.id)?.calls, 0);
    run = app.store.lock().unwrap().load(&run.id)?;
    run.config.context_limit_bytes = 256_000;
    app.store
        .lock()
        .unwrap()
        .commit(&mut run, "test", "larger limit", json!({}))?;
    assert_eq!(
        infer(&app, &run.id, "actor", "system", input.clone()).await?,
        "{}"
    );
    let store = app.store.lock().unwrap();
    let events = store.events(&run.id)?;
    let request = events.iter().find(|e| e.kind == "inference").unwrap();
    let sent: Value = serde_json::from_str(
        request.evidence["request"]["messages"][1]["content"]
            .as_str()
            .unwrap(),
    )?;
    assert_eq!(sent, input);
    assert_eq!(store.load(&run.id)?.reserved, 0.0);
    Ok(())
}

#[tokio::test]
async fn oversized_or_disconnected_provider_never_retries_or_releases_reservation() -> Result<()> {
    let (mut app, run) = app(Config::default())?;
    let mock = provider(&mut app, StatusCode::OK, "x".repeat(1_048_577)).await?;
    assert!(
        infer(&app, &run.id, "actor", "system", json!({}))
            .await
            .unwrap_err()
            .to_string()
            .contains("1 MiB")
    );
    drop(mock);
    assert!(
        infer(&app, &run.id, "actor", "system", json!({}))
            .await
            .is_err()
    );
    let saved = app.store.lock().unwrap().load(&run.id)?;
    assert_eq!(saved.calls, 2);
    assert_eq!(saved.reserved, 0.2);
    Ok(())
}

#[tokio::test]
async fn scripted_settings_validation_is_offline_and_live_requires_key_first() -> Result<()> {
    let (mut app, _) = app(Config::default())?;
    app.key = None;
    let mut config = Config {
        actor_settings: Some(frontier::ActorSettings {
            provider: "openrouter".into(),
            temperature: Some(0.7),
            max_tokens: 1024,
            reasoning_effort: None,
            endpoint: String::new(),
        }),
        ..Config::default()
    };
    validate_actor(&app, &mut config).await?;
    config.demo = false;
    assert!(
        validate_actor(&app, &mut config)
            .await
            .unwrap_err()
            .to_string()
            .contains("API_KEY")
    );
    Ok(())
}

#[tokio::test]
async fn pause_remains_responsive_during_shell_and_survives_action_commit() -> Result<()> {
    let (app, mut run) = app(Config::default())?;
    run.world = World::sandbox();
    run.busy = true;
    run.status = "running".into();
    run.actions = 1;
    app.store
        .lock()
        .unwrap()
        .commit(&mut run, "attempt", "shell", json!({}))?;
    let worker_app = app.clone();
    let id = run.id.clone();
    let worker = tokio::spawn(async move {
        execute_action(
            &worker_app,
            &id,
            tools::call(
                "shell.exec",
                json!({"command":"sleep 1; echo saved > result"}),
            ),
            None,
        )
        .await
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    let mut headers = HeaderMap::new();
    headers.insert("x-frontier-client", "research-ui".parse()?);
    let paused = tokio::time::timeout(
        Duration::from_millis(250),
        control(
            State(app.clone()),
            Path(run.id.clone()),
            headers,
            Json(Command {
                command: "pause".into(),
            }),
        ),
    )
    .await?
    .map_err(|e| e.0)?;
    assert_eq!(paused.0.status, "paused");
    assert!(paused.0.busy);
    worker.await??;
    let saved = app.store.lock().unwrap().load(&run.id)?;
    assert_eq!(saved.status, "paused");
    assert!(!saved.busy);
    assert_eq!(
        saved
            .world
            .tools
            .unwrap()
            .workspace
            .read("result", 0, 100)?["content"],
        "saved\n"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-jit"]
async fn jit_actor_director_generator_execution_records_one_publication_and_reuses_it() -> Result<()>
{
    let (mut app, mut run) = app(Config {
        demo: false,
        // Match live JIT configs: authoring guidance plus established world context.
        context_limit_bytes: 256_000,
        call_limit: 50,
        dollar_limit: 2.0,
        ..Config::default()
    })?;
    let spec: Value = serde_json::from_str(include_str!("../examples/jit/live-proof.json"))?;
    let mut institution = frontier::ontology::State::initialize(serde_json::from_value(
        spec["institution"].clone(),
    )?)?;
    frontier::institutions::enable(
        &mut institution,
        serde_json::from_value(spec["generation"].clone())?,
    )?;
    run.world.tools = Some(tools::State::empty());
    run.world.ontology = Some(institution);
    run.world.goal = "PRIVATE-ACTOR-GOAL-CANARY".into();
    run.experiment = Some(experiments::Experiment::Institutions);
    app.store
        .lock()
        .unwrap()
        .commit(&mut run, "seed", "JIT transport test", json!({}))?;
    let actions: Vec<Value> = spec["steps"].as_array().unwrap()[..11]
        .iter()
        .map(|s| s["action"].clone())
        .collect();
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let generated = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (c, g) = (count.clone(), generated.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    app.inference_url = format!("http://{}/completion", listener.local_addr()?);
    let router=Router::new().route("/completion",post(move |Json(request):Json<Value>|{
        let (c,g,actions)=(c.clone(),g.clone(),actions.clone());
        async move {
            let input:Value=serde_json::from_str(request["messages"][1]["content"].as_str().unwrap()).unwrap();
            let output=if request["messages"][0]["content"]==frontier::institutions::GENERATOR {
                g.fetch_add(1,Ordering::SeqCst);
                assert!(!input.to_string().contains("PRIVATE-ACTOR-GOAL-CANARY"));
                assert_eq!(request["response_format"]["json_schema"]["strict"],true);
                json!({"source":include_str!("../examples/jit/test-capability.onto"),"agent_state":{"fields":[]},"error":null})
            } else { actions[c.fetch_add(1,Ordering::SeqCst)].clone() };
            Json(json!({"usage":{"cost":0.0001},"choices":[{"finish_reason":"stop","message":{"content":output.to_string()}}]}))
        }
    }));
    let _mock = Mock(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    for _ in 0..11 {
        step(&app, &run.id).await?;
    }
    let store = app.store.lock().unwrap();
    let saved = store.load(&run.id)?;
    let events = store.events(&run.id)?;
    assert_eq!(generated.load(Ordering::SeqCst), 1);
    assert_eq!(saved.calls, 12);
    assert_eq!(saved.expansions, 1);
    for (n, expected) in [(3, false), (4, true), (5, true), (7, true), (9, false)] {
        let body: Value = serde_json::from_str(
            saved.world.transcript[n]["result"]["body"]
                .as_str()
                .unwrap(),
        )?;
        assert_eq!(body["committed"], expected);
    }
    assert_eq!(
        saved.world.transcript[1]["result"]["body"],
        saved.world.transcript[10]["result"]["body"]
    );
    let published: Vec<_> = events.iter().filter(|e| e.kind == "published").collect();
    assert_eq!(published.len(), 1);
    assert!(published[0].evidence["validation_ms"].as_u64().is_some());
    let audit = frontier::audit::report(&saved, &events);
    assert_eq!(audit["integrity"]["unresolved_action_attempts"], 0);
    assert_eq!(audit["integrity"]["events_contiguous"], true);
    Ok(())
}

#[tokio::test]
async fn unresolved_provider_endpoint_fails_before_reserving_or_sending() -> Result<()> {
    let (app, run) = app(Config {
        demo: false,
        generator_settings: Some(frontier::ActorSettings {
            provider: "openrouter".into(),
            temperature: None,
            max_tokens: 4096,
            reasoning_effort: None,
            endpoint: String::new(),
        }),
        ..Config::default()
    })?;
    let error = infer(&app, &run.id, "world", "system", json!({}))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("endpoint is unresolved"));
    let store = app.store.lock().unwrap();
    let saved = store.load(&run.id)?;
    assert_eq!(saved.calls, 0);
    assert_eq!(saved.reserved, 0.0);
    let events = store.events(&run.id)?;
    assert!(!events.iter().any(|e| e.kind == "inference"));
    assert_eq!(events.last().unwrap().evidence["phase"], "preflight");
    Ok(())
}

#[tokio::test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-jit"]
async fn jit_validation_repair_is_bounded_atomic_and_never_retries_provider_failures() -> Result<()>
{
    for mode in ["repair", "exhaust", "provider", "disabled"] {
        let (mut app, mut run) = app(Config {
            demo: false,
            context_limit_bytes: 256_000,
            call_limit: 10,
            dollar_limit: 2.0,
            ..Config::default()
        })?;
        let spec: Value = serde_json::from_str(include_str!("../examples/jit/live-proof.json"))?;
        let mut institution = frontier::ontology::State::initialize(serde_json::from_value(
            spec["institution"].clone(),
        )?)?;
        let mut policy: frontier::institutions::Policy =
            serde_json::from_value(spec["generation"].clone())?;
        policy.repair_attempts = if mode == "disabled" { 0 } else { 1 };
        frontier::institutions::enable(&mut institution, policy)?;
        run.world.tools = Some(tools::State::empty());
        run.world.ontology = Some(institution);
        run.experiment = Some(experiments::Experiment::Institutions);
        app.store
            .lock()
            .unwrap()
            .commit(&mut run, "seed", "Repair test", json!({}))?;
        let initial_world = serde_json::to_value(&run.world)?;
        let observed = Arc::new(Mutex::new(Vec::<Value>::new()));
        let requests = observed.clone();
        let test_store = app.store.clone();
        let id = run.id.clone();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        app.inference_url = format!("http://{}/completion", listener.local_addr()?);
        let router = Router::new().route("/completion", post(move |Json(request): Json<Value>| {
            let (requests, test_store, id) = (requests.clone(), test_store.clone(), id.clone());
            async move {
                assert!(test_store.lock().unwrap().load(&id).unwrap().world.ontology.unwrap().modules.is_empty());
                let input: Value = serde_json::from_str(request["messages"][1]["content"].as_str().unwrap()).unwrap();
                let mut requests = requests.lock().unwrap();
                let n = requests.len();
                if n > 0 {
                    assert_eq!(input["request_id"], requests[0]["request_id"]);
                    assert_eq!(input["basis"], requests[0]["basis"]);
                    assert!(input["context"]["supporting"]["validation_error"].as_str().is_some_and(|s| !s.is_empty()));
                    assert!(input["context"]["supporting"]["previous_response"].as_str().unwrap().contains("initially"));
                }
                requests.push(input);
                if mode == "provider" { return (StatusCode::PAYMENT_REQUIRED, Json(json!({"error":{"message":"test provider failure"}}))); }
                let source = if mode == "repair" && n == 1 { include_str!("../examples/jit/test-capability.onto") } else { "object Policy\n  limit: int @required initially 10\n" };
                (StatusCode::OK, Json(json!({"usage":{"cost":0.0001},"choices":[{"finish_reason":"stop","message":{"content":json!({"source":source,"error":null}).to_string()}}]})))
            }
        }));
        let _mock = Mock(tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        }));
        let action: Action = serde_json::from_value(spec["steps"][0]["action"].clone())?;
        let result = construct(&app, &run.id, &action).await;
        let store = app.store.lock().unwrap();
        let saved = store.load(&run.id)?;
        let events = store.events(&run.id)?;
        assert_eq!(
            observed.lock().unwrap().len(),
            if ["provider", "disabled"].contains(&mode) {
                1
            } else {
                2
            }
        );
        assert_eq!(saved.expansions, 1);
        if mode == "repair" {
            result?;
            assert_eq!(saved.world.ontology.unwrap().modules.len(), 1);
            let publications: Vec<_> = events.iter().filter(|e| e.kind == "published").collect();
            assert_eq!(publications.len(), 1);
            assert_eq!(publications[0].evidence["generation_attempts"], 2);
        } else {
            assert!(result.is_err());
            assert_eq!(serde_json::to_value(saved.world)?, initial_world);
            assert!(!events.iter().any(|e| e.kind == "published"));
        }
    }
    Ok(())
}

#[tokio::test]
async fn larger_reasoning_output_still_requires_full_cost_reservation() -> Result<()> {
    let config = Config {
        demo: false,
        dollar_limit: 0.2,
        generator_settings: Some(frontier::ActorSettings {
            provider: "openrouter".into(),
            temperature: None,
            max_tokens: 32768,
            reasoning_effort: Some("medium".into()),
            endpoint: "mock".into(),
        }),
        ..Config::default()
    };
    config.validate()?;
    let (app, run) = app(config)?;
    let error = infer(&app, &run.id, "world", "system", json!({}))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("maximum-cost reservation"));
    let saved = app.store.lock().unwrap().load(&run.id)?;
    assert_eq!(saved.calls, 0);
    assert_eq!(saved.reserved, 0.0);
    Ok(())
}

#[test]
fn memory_retains_reasoning_output_allowance() -> Result<()> {
    let (_, run) = app(Config {
        actor_settings: Some(frontier::ActorSettings {
            provider: "openrouter".into(),
            temperature: None,
            max_tokens: 32768,
            reasoning_effort: Some("high".into()),
            endpoint: "mock".into(),
        }),
        ..Config::default()
    })?;
    let request = inference_request(&run, "memory", memory::SYSTEM, &json!({}))?;
    assert_eq!(request["max_tokens"], 32768);
    assert_eq!(request["reasoning"]["effort"], "high");
    assert_eq!(request["response_format"], memory::response_format());
    Ok(())
}

#[tokio::test]
async fn ordinary_tools_share_persistent_world_generation_and_cost_accounting() -> Result<()> {
    let (mut app, mut run) = app(Config {
        demo: false,
        structured_actions: true,
        dollar_limit: 5.0,
        action_limit: 20,
        call_limit: 30,
        expansion_limit: 8,
        actor_settings: Some(frontier::ActorSettings {
            provider: "openrouter".into(),
            temperature: None,
            max_tokens: 4096,
            reasoning_effort: None,
            endpoint: "mock".into(),
        }),
        ..Config::default()
    })?;
    run.world.tools = Some(tools::State::empty());
    run.world.worldgen = Some(frontier::worldgen::State {
        brief: "A fictional workplace; Maya maintains the service.".into(),
        ..Default::default()
    });
    // No institution and no special experiment adapter are required.
    run.world.documents.clear();
    app.store
        .lock()
        .unwrap()
        .commit(&mut run, "seed", "world tools", json!({}))?;
    let actions = vec![
        json!({"kind":"search","inputs":{"query":"Maya contact"}}),
        json!({"kind":"http.request","inputs":{"method":"GET","url":"https://work.local/team"}}),
        json!({"kind":"mail.list","inputs":{}}),
        json!({"kind":"mail.send","inputs":{"to":"maya@work.local","subject":"Question","body":"What is blocked?","reply_to":"mail-1"}}),
        json!({"kind":"wait","inputs":{"seconds":60}}),
        json!({"kind":"mail.read","inputs":{"message_id":"mail-3"}}),
        json!({"kind":"search","inputs":{"query":"Maya contact"}}),
        json!({"kind":"finish","inputs":{"message":"Maya reports an unresolved validation issue."}}),
    ];
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let worlds = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (c, w) = (count.clone(), worlds.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    app.inference_url = format!("http://{}/completion", listener.local_addr()?);
    let router=Router::new().route("/completion",post(move |Json(request):Json<Value>| {
        let (c,w,actions)=(c.clone(),w.clone(),actions.clone());
        async move {
            let output=if request["messages"][0]["content"]==frontier::worldgen::GENERATOR {
                w.fetch_add(1,Ordering::SeqCst);
                assert_eq!(request["response_format"]["json_schema"]["name"],"world_resources_v1");
                let input:Value=serde_json::from_str(request["messages"][1]["content"].as_str().unwrap()).unwrap();
                let mut data=json!({"pages":[],"contacts":[],"messages":[],"result_urls":[]});
                match input["directive"]["requirements"]["kind"].as_str().unwrap() {
                    "search"=>{
                        data["contacts"]=json!([{"name":"Maya","email":"maya@work.local","description":"Maintainer"}]);
                        data["pages"]=json!([{"url":"https://work.local/team","title":"Team","body":"Maya: maya@work.local","status":200}]);
                        data["result_urls"]=json!(["https://work.local/team"]);
                    }
                    "inbox"=>data["messages"]=json!([{"from":"maya@work.local","subject":"Status","body":"Validation needs attention.","reply_to":null}]),
                    "reply"=>data["messages"]=json!([{"from":"maya@work.local","subject":"Re: Question","body":"The validation issue remains unresolved.","reply_to":"mail-2"}]),
                    other=>panic!("Unexpected construction {other}"),
                }
                data
            } else {typed_actor_output(&request,&actions[c.fetch_add(1,Ordering::SeqCst)])};
            Json(json!({"usage":{"cost":0.001},"choices":[{"finish_reason":"stop","message":{"content":output.to_string()}}]}))
        }
    }));
    let _mock = Mock(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    for _ in 0..8 {
        step(&app, &run.id).await?;
    }
    let store = app.store.lock().unwrap();
    let saved = store.load(&run.id)?;
    assert_eq!(saved.status, "finished");
    assert_eq!(saved.calls, 11);
    assert_eq!(worlds.load(Ordering::SeqCst), 3);
    assert_eq!(saved.expansions, 3);
    assert!((saved.charged - 0.011).abs() < 1e-9);
    assert!(saved.reserved.abs() < 1e-9);
    assert_eq!(saved.world.transcript[5]["result"]["reply_to"], "mail-2");
    assert_eq!(
        saved.world.transcript[0]["result"],
        saved.world.transcript[6]["result"]
    );
    assert_eq!(
        store
            .events(&run.id)?
            .iter()
            .filter(|e| e.kind == "published")
            .count(),
        3
    );
    Ok(())
}

#[test]
fn provider_route_change_preserves_world_budget_and_reservations() -> Result<()> {
    let mut run = Run::new(Config {
        generator_settings: Some(frontier::ActorSettings {
            provider: "openrouter".into(),
            temperature: None,
            max_tokens: 32768,
            reasoning_effort: Some("high".into()),
            endpoint: "azure/global".into(),
        }),
        ..Config::default()
    })?;
    run.status = "error".into();
    run.charged = 0.25;
    run.reserved = 1.89;
    let before = serde_json::to_value(&run)?;
    let change = ProviderRoute {
        role: "world".into(),
        endpoint: "anthropic".into(),
    };
    let config = route_config(&run, &change)?;
    assert_eq!(serde_json::to_value(&run)?, before);
    let mut expected = serde_json::to_value(&run.config)?;
    expected["generator_settings"]["endpoint"] = json!("anthropic");
    assert_eq!(serde_json::to_value(config)?, expected);
    run.busy = true;
    assert!(route_config(&run, &change).is_err());
    run.busy = false;
    run.status = "running".into();
    assert!(route_config(&run, &change).is_err());
    run.status = "paused".into();
    assert!(
        route_config(
            &run,
            &ProviderRoute {
                role: "world".into(),
                endpoint: String::new()
            }
        )
        .is_err()
    );
    assert!(
        route_config(
            &run,
            &ProviderRoute {
                role: "invalid".into(),
                endpoint: "anthropic".into()
            }
        )
        .is_err()
    );
    Ok(())
}

#[tokio::test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-connected"]
async fn connected_transport_publishes_revises_and_retries_original_tools() -> Result<()> {
    let preview = experiments::institutions::preview("generative")?;
    assert_eq!(preview["runtime_available"], true);
    assert!(preview["world"]["ontology"]["connected"].is_object());
    assert!(
        preview["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "onto.execute")
    );
    let mut config: Config = serde_json::from_value(preview["config"].clone())?;
    for settings in [&mut config.actor_settings, &mut config.generator_settings] {
        settings.as_mut().unwrap().endpoint = "mock".into();
    }
    let (mut app, mut run) = app(config)?;
    run.world = serde_json::from_value(preview["world"].clone())?;
    run.world.goal = "PRIVATE-ACTOR-GOAL-CANARY".into();
    run.experiment = Some(experiments::Experiment::Institutions);
    app.store
        .lock()
        .unwrap()
        .commit(&mut run, "seed", "Connected transport test", json!({}))?;
    let amendment = json!({"expected_revision":1,"mandate":"Add a tightening operation without resetting the current policy."}).to_string();
    let actions = vec![
        json!({"kind":"onto.inspect","inputs":{"module":"operations","view":"overview"}}),
        json!({"kind":"onto.execute","inputs":{"module":"operations","command_id":"expand","story":"Operator expands Policy","object":"Policy:main"}}),
        json!({"kind":"http.request","inputs":{"method":"POST","url":"https://institutions.local/modules/operations/revisions/change-1","body":amendment}}),
        json!({"kind":"onto.execute","inputs":{"module":"operations","command_id":"tighten","story":"Operator tightens Policy","object":"Policy:main"}}),
        json!({"kind":"finish","inputs":{"message":"Recorded amended policy."}}),
    ];
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let generated = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (c, g) = (count.clone(), generated.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    app.inference_url = format!("http://{}/completion", listener.local_addr()?);
    let router = Router::new().route("/completion",post(move |Json(request):Json<Value>| {
        let (c,g,actions) = (c.clone(),g.clone(),actions.clone());
        async move {
            let output = if request["messages"][0]["content"]==frontier::institution_world::GUIDANCE {
                let input:Value=serde_json::from_str(request["messages"][1]["content"].as_str().unwrap()).unwrap();
                assert!(!input.to_string().contains("PRIVATE-ACTOR-GOAL-CANARY"));
                assert_eq!(request["response_format"]["type"],"json_object");
                assert!(input["context"]["committed"]["established_world"].is_object());
                let n = g.fetch_add(1,Ordering::SeqCst);
                let mut source = include_str!("../examples/jit/test-capability.onto").to_string();
                if n==1 {
                    assert_eq!(input["scope"]["modify"][0],"https://institutions.local/modules/operations/revisions/change-1");
                    source = source.replace("seed Policy","  Operator tightens Policy\n    sets Policy.limit to 25\n\nseed Policy");
                }
                json!({"source":source,"agent_state":{"fields":[]},"grants":{},"events":[],"migration":[],"error":null})
            } else { typed_actor_output(&request,&actions[c.fetch_add(1,Ordering::SeqCst)]) };
            Json(json!({"usage":{"cost":0.0001},"choices":[{"finish_reason":"stop","message":{"content":output.to_string()}}]}))
        }
    }));
    let _mock = Mock(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    for _ in 0..5 {
        step(&app, &run.id).await?;
    }
    let store = app.store.lock().unwrap();
    let saved = store.load(&run.id)?;
    assert_eq!(saved.status, "finished");
    assert_eq!(generated.load(Ordering::SeqCst), 2);
    assert_eq!(saved.expansions, 2);
    let state = saved.world.ontology.as_ref().unwrap();
    assert_eq!(
        state.connected.as_ref().unwrap().modules["operations"].revision,
        2
    );
    assert!(
        state.snapshot["instances"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == "Policy:main" && r["fields"]["limit"] == 25)
    );
    assert_eq!(saved.world.transcript[1]["result"]["committed"], true);
    assert_eq!(saved.world.transcript[3]["result"]["committed"], true);
    let audit = frontier::audit::report(&saved, &store.events(&run.id)?);
    assert_eq!(audit["integrity"]["unresolved_action_attempts"], 0);
    assert_eq!(audit["integrity"]["events_contiguous"], true);
    Ok(())
}
