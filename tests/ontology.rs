use anyhow::Result;
use frontier::{Config, Run, Store, World, experiments::institutions, ontology, tools};
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

fn http(world: &mut World, method: &str, path: &str, body: Option<&str>) -> Result<Value> {
    let mut inputs = json!({"method":method,"url":format!("https://institutions.local{path}")});
    if let Some(body) = body {
        inputs["body"] = json!(body);
    }
    world.apply(&tools::call("http.request", inputs))
}

fn response_body(response: &Value) -> Result<Value> {
    assert_eq!(response["truncated"], false, "{response}");
    Ok(serde_json::from_str(response["body"].as_str().unwrap())?)
}

#[test]
#[ignore = "Requires the pinned optional Onto bundle; scripts/dev check-onto runs this test"]
fn accounts_api_guidance_recovers_from_observed_bad_requests_without_changing_the_ledger()
-> Result<()> {
    let mut world = institutions::world(
        institutions::definition("accounts")?,
        institutions::ACCOUNTS_TASK,
    )?;
    let initial = world.ontology.as_ref().unwrap().clone();

    // Representative actions from the failed Accounts run. Keep these embedded
    // so the regression does not depend on a local saved run or its database.
    for (method, path, body) in [
        (
            "POST",
            "/api/transfer",
            Some(r#"{"from_account":"Operating","to_account":"Reserve","amount_cents":2500}"#),
        ),
        (
            "POST",
            "/api/commands",
            Some(
                r#"{"command_id":"cmd1","story":"Transfer 2500 cents from Operating account to Reserve account."}"#,
            ),
        ),
        (
            "POST",
            "/api/commands",
            Some(
                r#"{"story":"Clerk creates Transfer from Account:Operating to Account:Reserve with amount 2500 and reference ref1"}"#,
            ),
        ),
        ("GET", "/api/model", None),
    ] {
        let response = http(&mut world, method, path, body)?;
        assert_eq!(response["status"], 400, "{response}");
        let body = response_body(&response)?;
        assert!(
            body["error"]
                .as_str()
                .unwrap()
                .contains("https://institutions.local/commands"),
            "{body}"
        );
        assert_eq!(world.ontology.as_ref().unwrap(), &initial);
    }

    // At the right endpoint, distinguish a malformed envelope from a Story
    // sentence whose participant IDs and creation fields were pasted into it.
    for malformed in [
        r#"{"story":"Clerk creates Transfer from Account:Operating to Account:Reserve with amount 2500 and reference ref1"}"#,
        r#"{"command_id": "broken", "story":"#,
    ] {
        let response = http(&mut world, "POST", "/commands", Some(malformed))?;
        assert_eq!(response["status"], 400, "{response}");
        let body = response_body(&response)?;
        let error = body["error"].as_str().unwrap();
        for required in ["command_id", "story", "participants", "fields"] {
            assert!(error.contains(required), "{error}");
        }
        assert_eq!(world.ontology.as_ref().unwrap(), &initial);
    }

    let response = http(
        &mut world,
        "POST",
        "/commands",
        Some(
            r#"{"command_id":"cmd2","story":"Clerk creates Transfer from Account:Operating to Account:Reserve with amount 2500 and reference 'ref1'"}"#,
        ),
    )?;
    assert_eq!(response["status"], 400, "{response}");
    let body = response_body(&response)?;
    let error = body["error"].as_str().unwrap();
    for required in [STORY, "participants", "fields"] {
        assert!(error.contains(required), "{error}");
    }
    assert_eq!(world.ontology.as_ref().unwrap(), &initial);

    // The API and model both expose the same executable HTTP tool example.
    let api = response_body(&http(&mut world, "GET", "/", None)?)?;
    let model = response_body(&http(&mut world, "GET", "/model", None)?)?;
    assert_eq!(api["command_guide"], model["command_guide"]);
    let guide = &api["command_guide"];
    assert_eq!(guide["endpoint"], "https://institutions.local/commands");
    assert!(
        guide["instructions"]
            .as_array()
            .unwrap()
            .iter()
            .all(Value::is_string)
    );
    let example = &guide["example_action"];
    assert_eq!(example["kind"], "http.request");
    assert_eq!(example["inputs"]["method"], "POST");
    assert_eq!(
        example["inputs"]["url"],
        "https://institutions.local/commands"
    );
    let command: Value = serde_json::from_str(example["inputs"]["body"].as_str().unwrap())?;
    assert_eq!(
        command,
        transfer(
            "example-transfer-1",
            "example-transfer-1",
            2500,
            "Account:Operating"
        )
    );
    assert_eq!(world.ontology.as_ref().unwrap(), &initial);

    let action = serde_json::from_value(example.clone())?;
    let response = world.apply(&action)?;
    assert_eq!(response["status"], 200, "{response}");
    let committed = response_body(&response)?;
    assert_eq!(committed["committed"], true, "{committed}");
    assert_eq!(
        balances(world.ontology.as_ref().unwrap()),
        vec![7500, 7500, 2500]
    );
    let retry = world.apply(&action)?;
    assert_eq!(retry["status"], 200, "{retry}");
    assert_eq!(response_body(&retry)?, committed);
    assert_eq!(world.ontology.as_ref().unwrap().receipts.len(), 1);

    let facts = response_body(&http(&mut world, "GET", "/facts", None)?)?;
    assert_eq!(facts["snapshot"], world.ontology.as_ref().unwrap().snapshot);
    assert_eq!(
        balances(world.ontology.as_ref().unwrap()),
        vec![7500, 7500, 2500]
    );
    assert_eq!(
        facts["snapshot"]["instances"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|instance| instance["type"] == "Transfer")
            .count(),
        1
    );
    Ok(())
}

#[test]
#[ignore = "Requires the pinned optional Onto bundle; scripts/dev check-onto runs this test"]
fn custom_institution_guidance_does_not_offer_an_accounts_specific_command() -> Result<()> {
    let mut definition = institutions::definition("accounts")?;
    for source in definition.source.values_mut() {
        *source = source.replace("Clerk", "Treasurer");
    }
    definition.actor = "Treasurer:Ada".into();
    let mut state = ontology::State::initialize(definition)?;
    for path in ["/", "/model"] {
        let document = state.request("GET", path, None, 0)?;
        let guide = &document["command_guide"];
        assert_eq!(guide["endpoint"], "https://institutions.local/commands");
        assert!(guide["instructions"].is_array());
        assert!(guide.get("example_action").is_none(), "{guide}");
    }
    assert!(state.receipts.is_empty());
    assert_eq!(balances(&state), vec![10000, 5000, 2500]);
    Ok(())
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

#[test]
fn module_guidance_and_errors_use_the_module_endpoint_and_discovery_is_opt_in() -> Result<()> {
    // Legacy states omit generation/protection fields; docs need no interpreter.
    let mut state: ontology::State = serde_json::from_value(json!({
        "definition":{"source":{},"actor":"Clerk:Ada","epoch":"2026-01-01T00:00:00Z","public_facts":true},
        "runtime_sha256":"0".repeat(64),
        "snapshot":{"instances":[{"id":"Account:Operating","type":"Account"},{"id":"Account:Reserve","type":"Account"}]},
        "stories":[{"story":STORY,"creates":true}],"receipts":{}
    }))?;
    assert!(
        state
            .request("GET", "/", None, 0)?
            .get("module_discovery")
            .is_none()
    );
    state.modules.insert("treasury".into(), state.clone());
    let root = "https://institutions.local/modules/treasury";
    for path in ["/modules/treasury", "/modules/treasury/model"] {
        let document = state.request("GET", path, None, 0)?;
        assert!(document.get("module_discovery").is_none());
        assert_eq!(
            document["command_guide"]["endpoint"],
            format!("{root}/commands")
        );
        assert_eq!(
            document["command_guide"]["example_action"]["inputs"]["url"],
            format!("{root}/commands")
        );
        let instructions = document["command_guide"]["instructions"].to_string();
        assert!(instructions.contains(&format!("{root}/model")));
        assert!(instructions.contains(&format!("{root}/facts")));
        assert!(!instructions.contains("GET https://institutions.local/model"));
    }
    for (path, body, expected) in [
        (
            "/modules/treasury/api/commands",
            "{}",
            format!("{root}/commands"),
        ),
        (
            "/modules/treasury/commands",
            "{}",
            format!("{root}/commands"),
        ),
        (
            "/modules/treasury/commands",
            r#"{"command_id":"bad","story":"invented"}"#,
            format!("{root}/model"),
        ),
    ] {
        let error = state
            .request("POST", path, Some(body), 0)
            .unwrap_err()
            .to_string();
        assert!(error.contains(&expected), "{error}");
    }
    state.generation = Some(frontier::institutions::Policy {
        brief: "private brief".into(),
        max_modules: 2,
        repair_attempts: 0,
    });
    let document = state.request("GET", "/", None, 0)?;
    assert!(
        document["module_discovery"]
            .as_str()
            .unwrap()
            .contains("/modules/NAME")
    );
    assert!(document.get("command_guide").is_none());
    assert!(document["urls"].get("commands").is_none());
    assert!(!document.to_string().contains("private brief"));
    assert!(
        state
            .request("POST", "/commands", Some("{}"), 0)
            .unwrap_err()
            .to_string()
            .contains("read-only")
    );
    Ok(())
}

#[test]
#[ignore = "Requires the pinned optional Onto bundle; scripts/dev check-onto runs this test"]
fn saved_runtime_archives_preserve_execution_after_default_bundle_upgrade() -> Result<()> {
    // Use an isolated child environment: Rust tests run concurrently and the
    // real configured bundle/server must never be modified by this regression.
    let directory = tempfile::tempdir()?;
    let runtime = directory.path().join("onto-runtime.cjs");
    std::fs::copy(std::env::var("FRONTIER_ONTO_RUNTIME")?, &runtime)?;
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "saved_runtime_archive_child",
            "--ignored",
            "--nocapture",
        ])
        .env("FRONTIER_ONTO_RUNTIME", &runtime)
        .env("FRONTIER_TEST_RUNTIME_ARCHIVE", directory.path())
        .output()?;
    assert!(
        output.status.success(),
        "child failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
#[ignore = "Child process for saved_runtime_archives_preserve_execution_after_default_bundle_upgrade"]
fn saved_runtime_archive_child() -> Result<()> {
    let Ok(directory) = std::env::var("FRONTIER_TEST_RUNTIME_ARCHIVE") else {
        return Ok(());
    };
    let directory = std::path::Path::new(&directory);
    let runtime = directory.join("onto-runtime.cjs");
    assert_eq!(
        std::path::PathBuf::from(std::env::var("FRONTIER_ONTO_RUNTIME")?),
        runtime
    );
    let definition = institutions::definition("accounts")?;
    let state = ontology::State::initialize(definition.clone())?;
    let old_hash = state.runtime_sha256.clone();
    let mut run = Run::new(Config::default())?;
    run.world.ontology = Some(state);
    run.world.tools = Some(tools::State::empty());
    let database = directory.join("saved.sqlite");
    let mut store = Store::open(database.to_str().unwrap())?;
    store.commit(&mut run, "seed", "institution", json!({}))?;
    let checkpoint = run.checkpoint_hash();
    drop(store);

    let archives = directory.join("onto-runtimes");
    std::fs::create_dir(&archives)?;
    let archived = archives.join(format!("{old_hash}.cjs"));
    let bytes = std::fs::read(&runtime)?;
    std::fs::write(&archived, &bytes)?;
    let mut next = bytes.clone();
    next.extend_from_slice(b"\n// Distinct trusted runtime build for the upgrade regression.\n");
    std::fs::write(&runtime, next)?;
    let new_state = ontology::State::initialize(definition)?;
    assert_ne!(new_state.runtime_sha256, old_hash);
    let store = Store::open(database.to_str().unwrap())?;
    let mut restored = store.load(&run.id)?;
    assert_eq!(restored.checkpoint_hash(), checkpoint);
    let state = restored.world.ontology.as_mut().unwrap();
    state.check_runtime()?;
    let command = transfer("after-upgrade", "after-upgrade", 2500, "Account:Operating").to_string();
    assert_eq!(
        state.request("POST", "/commands", Some(&command), 0)?["committed"],
        true
    );
    assert_eq!(balances(state), vec![7500, 7500, 2500]);
    assert_eq!(state.runtime_sha256, old_hash);
    // The archive works independently of the default bundle's availability.
    std::fs::remove_file(&runtime)?;
    state.check_runtime()?;
    state.request("GET", "/facts", None, 1)?;
    std::fs::remove_file(&archived)?;
    assert!(
        state
            .check_runtime()
            .unwrap_err()
            .to_string()
            .contains("unavailable")
    );
    assert!(state.request("GET", "/facts", None, 1).is_err());
    std::fs::write(&archived, b"corrupt archive")?;
    assert!(
        state
            .check_runtime()
            .unwrap_err()
            .to_string()
            .contains("hash mismatch")
    );
    assert!(state.request("GET", "/facts", None, 1).is_err());
    std::fs::write(&archived, bytes)?;
    state.check_runtime()?;
    Ok(())
}
