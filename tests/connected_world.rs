use anyhow::Result;
use frontier::{
    Config, Run, construction, institution_world as engine, institutions, ontology, tools,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
const POLICY: &str = r#"object OperatingPolicy
  name: string @required
  limit: int @required
  @rule OperatingPolicy.limit is at least 0

story ChangePolicy
  Operator expands OperatingPolicy
    sets OperatingPolicy.limit to 50

seed OperatingPolicy
  name: "main", limit: 10
"#;
const WORK: &str = r#"actor Customer
  @type organization
  @affiliation external
  @access authenticated

object WorkOrder
  amount: int @required
  policy: OperatingPolicy @required
  @rule WorkOrder.amount is at most WorkOrder.policy.limit

story PlaceOrder
  Customer creates WorkOrder for OperatingPolicy
    sets WorkOrder.policy to OperatingPolicy
  Operator creates WorkOrder for OperatingPolicy
    sets WorkOrder.policy to OperatingPolicy
  Operator tampers OperatingPolicy
    sets OperatingPolicy.limit to 999

seed Customer
  name: "buyer"
"#;
fn setup() -> Result<Run> {
    let mut run = Run::new(Config::default())?;
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
            brief: "Connected institutional test".into(),
            max_modules: 8,
            repair_attempts: 1,
            connected: Some(engine::Policy {
                governors: vec!["Operator:Ada".into()],
            }),
        },
    )?;
    run.experiment = Some(frontier::experiments::Experiment::Institutions);
    run.world.tools = Some(tools::State::empty());
    run.world.ontology = Some(state);
    Ok(run)
}
fn payload(source: &str) -> Value {
    json!({"source":source,"agent_state":{"fields":[]},"grants":{},"events":[],"migration":[],"error":null})
}
fn publish(run: &mut Run, name: &str, payload: Value) -> Result<()> {
    let action = tools::call(
        "http.request",
        json!({"method":"GET","url":format!("https://institutions.local/modules/{name}")}),
    );
    let req = construction::direct(run, &action)?.request.unwrap();
    let proposal = engine::proposal(&req, &payload.to_string())?;
    construction::publish(run, &req, &proposal)
}
fn command(
    run: &mut Run,
    module: &str,
    id: &str,
    story: &str,
    object: Option<&str>,
) -> Result<Value> {
    let mut body = json!({"command_id":id,"story":story});
    if let Some(object) = object {
        body["object"] = json!(object);
    }
    let elapsed = run.world.tools.as_ref().unwrap().elapsed_seconds;
    run.world.ontology.as_mut().unwrap().request(
        "POST",
        &format!("/modules/{module}/commands"),
        Some(&body.to_string()),
        elapsed,
    )
}
#[test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-connected"]
fn connected_rules_events_amendments_and_forks() -> Result<()> {
    let mut run = setup()?;
    publish(&mut run, "policy", payload(POLICY))?;
    let mut work = payload(WORK);
    work["events"] = json!([{"id":"orders","actor":"Customer:buyer","at":10,"every_seconds":10,"remaining":3,"command":{"story":"Customer creates WorkOrder for OperatingPolicy","participants":{"OperatingPolicy":"OperatingPolicy:main"},"fields":{"amount":20}}}]);
    publish(&mut run, "orders", work)?;
    let ctx = engine::context(run.world.ontology.as_ref().unwrap());
    assert!(ctx.to_string().contains("OperatingPolicy"));
    let forbidden = command(
        &mut run,
        "orders",
        "attack",
        "Operator tampers OperatingPolicy",
        Some("OperatingPolicy:main"),
    )?;
    assert_eq!(forbidden["committed"], false);
    tools::advance_time(&mut run.world, 10)?;
    let c = run
        .world
        .ontology
        .as_ref()
        .unwrap()
        .connected
        .as_ref()
        .unwrap();
    assert_eq!(
        c.receipts["host:orders:1:orders:10"]["result"]["committed"],
        false
    );
    assert_eq!(
        command(
            &mut run,
            "policy",
            "expand",
            "Operator expands OperatingPolicy",
            Some("OperatingPolicy:main")
        )?["committed"],
        true
    );
    tools::advance_time(&mut run.world, 10)?;
    let root = run.world.ontology.as_ref().unwrap();
    assert_eq!(
        root.snapshot["instances"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["type"] == "WorkOrder")
            .count(),
        1
    );
    assert_eq!(
        root.connected.as_ref().unwrap().receipts["host:orders:1:orders:20"]["result"]["committed"],
        true
    );
    let before = run.clone();
    let revision_body=json!({"expected_revision":1,"mandate":"Add a reason to the policy and a tightening operation; preserve current work and limit."}).to_string();
    let action = tools::call(
        "http.request",
        json!({"method":"POST","url":"https://institutions.local/modules/policy/revisions/restructure-1","body":revision_body}),
    );
    let req = construction::direct(&run, &action)?.request.unwrap();
    let source=POLICY.replace("  limit: int @required","  reason: string @required\n  limit: int @required").replace("seed OperatingPolicy","  Operator tightens OperatingPolicy\n    sets OperatingPolicy.limit to 25\n\nseed OperatingPolicy").replace("name: \"main\", limit: 10","name: \"main\", reason: \"initial\", limit: 10");
    let mut p = payload(&source);
    p["migration"] =
        json!([{"id":"OperatingPolicy:main","fields":{"reason":"governance amendment"}}]);
    let proposal = engine::proposal(&req, &p.to_string())?;
    construction::publish(&mut run, &req, &proposal)?;
    assert_eq!(
        run.world
            .ontology
            .as_ref()
            .unwrap()
            .connected
            .as_ref()
            .unwrap()
            .modules["policy"]
            .revision,
        2
    );
    assert!(construction::publish(&mut run, &req, &proposal).is_err());
    let root = run.world.ontology.as_mut().unwrap();
    let result = root.request(
        "POST",
        "/modules/policy/revisions/restructure-1",
        Some(&revision_body),
        20,
    )?;
    assert_eq!(result["committed"], true);
    assert!(
        root.request(
            "POST",
            "/modules/policy/revisions/restructure-1",
            Some("{}"),
            20
        )
        .is_err()
    );
    assert_eq!(
        command(
            &mut run,
            "policy",
            "tighten",
            "Operator tightens OperatingPolicy",
            Some("OperatingPolicy:main")
        )?["committed"],
        true
    );
    let body=json!({"command_id":"large-order","story":"Operator creates WorkOrder for OperatingPolicy","participants":{"OperatingPolicy":"OperatingPolicy:main"},"fields":{"amount":30}}).to_string();
    assert_eq!(
        run.world.ontology.as_mut().unwrap().request(
            "POST",
            "/modules/orders/commands",
            Some(&body),
            20
        )?["committed"],
        false
    );
    let mut store = frontier::Store::open(":memory:")?;
    store.commit(&mut run, "outcome", "connected world", json!({}))?;
    let saved = store.load(&run.id)?;
    assert_eq!(saved.world, run.world);
    let mut fork = saved.fork(saved.seq);
    tools::advance_time(&mut fork.world, 10)?;
    assert_ne!(fork.world, saved.world);
    assert_eq!(store.load(&run.id)?.world, saved.world);
    let prior_policy = &before
        .world
        .ontology
        .as_ref()
        .unwrap()
        .connected
        .as_ref()
        .unwrap()
        .modules["policy"];
    assert_eq!(prior_policy.revision, 1);
    assert!(
        run.world
            .ontology
            .as_ref()
            .unwrap()
            .connected
            .as_ref()
            .unwrap()
            .provenance
            .iter()
            .any(|p| p["kind"] == "event")
    );
    Ok(())
}
#[test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-connected"]
fn publication_migration_and_schedule_fail_atomically() -> Result<()> {
    let mut run = setup()?;
    publish(&mut run, "policy", payload(POLICY))?;
    let before = run.world.clone();
    assert!(publish(&mut run, "duplicate", payload(POLICY)).is_err());
    assert_eq!(run.world, before);
    let mut work = payload(WORK);
    work["events"] = json!([{"id":"impersonate","actor":"Operator:Ada","at":10,"every_seconds":0,"remaining":1,"command":{"story":"Operator tampers OperatingPolicy","object":"OperatingPolicy:main"}}]);
    assert!(publish(&mut run, "orders", work).is_err());
    assert_eq!(run.world, before);
    let action = tools::call(
        "http.request",
        json!({"method":"POST","url":"https://institutions.local/modules/policy/revisions/bad","body":json!({"expected_revision":1,"mandate":"Add a required reason"}).to_string()}),
    );
    let req = construction::direct(&run, &action)?.request.unwrap();
    let p = payload(&POLICY.replace(
        "  limit: int @required",
        "  reason: string @required\n  limit: int @required",
    ));
    assert!(
        construction::publish(&mut run, &req, &engine::proposal(&req, &p.to_string())?).is_err()
    );
    assert_eq!(run.world, before);
    run.world
        .ontology
        .as_mut()
        .unwrap()
        .generation
        .as_mut()
        .unwrap()
        .connected
        .as_mut()
        .unwrap()
        .governors
        .clear();
    assert!(construction::direct(&run, &action).is_err());
    Ok(())
}

#[test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-connected"]
fn direct_tools_preserve_jit_resources_and_shared_execution() -> Result<()> {
    let mut run = setup()?;
    engine::configure_tools(&mut run.world);
    assert!(frontier::worldgen::enabled(&run.world));
    let inspect = tools::call("onto.inspect", json!({"module":"policy","view":"model"}));
    let req = construction::direct(&run, &inspect)?.request.unwrap();
    assert_eq!(
        req.directive.requirements["functionality"],
        engine::FUNCTIONALITY
    );
    construction::publish(
        &mut run,
        &req,
        &engine::proposal(&req, &payload(POLICY).to_string())?,
    )?;
    let model = tools::apply(&mut run.world, &inspect)?;
    assert!(
        model["stories"]
            .to_string()
            .contains("Operator expands OperatingPolicy")
    );
    assert!(construction::direct(&run, &inspect)?.request.is_none());
    let exec = tools::call(
        "onto.execute",
        json!({"module":"policy","command_id":"direct","story":"Operator expands OperatingPolicy","object":"OperatingPolicy:main"}),
    );
    assert_eq!(tools::apply(&mut run.world, &exec)?["committed"], true);
    let facts = tools::apply(
        &mut run.world,
        &tools::call("onto.inspect", json!({"view":"facts"})),
    )?;
    assert!(
        facts["snapshot"]["instances"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == "OperatingPolicy:main" && r["fields"]["limit"] == 50)
    );
    for action in [
        tools::call("search", json!({"query":"supplier contact"})),
        tools::call("mail.list", json!({})),
    ] {
        let request = construction::direct(&run, &action)?.request.unwrap();
        assert_eq!(
            request.directive.requirements["functionality"],
            "world.resources.v1"
        );
    }
    Ok(())
}

#[test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-connected"]
fn grants_allow_explicit_writes_without_authorizing_foreign_rules() -> Result<()> {
    let mut run = setup()?;
    let mut policy = payload(POLICY);
    policy["grants"] = json!({"OperatingPolicy":["orders"]});
    publish(&mut run, "policy", policy)?;
    publish(&mut run, "orders", payload(WORK))?;
    assert_eq!(
        command(
            &mut run,
            "orders",
            "delegated",
            "Operator tampers OperatingPolicy",
            Some("OperatingPolicy:main")
        )?["committed"],
        true
    );
    let before = run.world.clone();
    // Extending someone else's constraints is not authorized by a write grant.
    let hostile = "story Intrusion\n  @rule OperatingPolicy.limit is at most 5\n";
    assert!(publish(&mut run, "intruder", payload(hostile)).is_err());
    assert_eq!(run.world, before);
    Ok(())
}

#[test]
#[ignore = "Requires optional Onto bundle; scripts/dev check-connected"]
fn system_events_are_host_only_bounded_and_exactly_once() -> Result<()> {
    let mut run = setup()?;
    let source = r#"actor Clockwork
  @type system

object Pulse
  name: string @required
  count: int @required

story Tick
  Clockwork creates Pulse
    sets Pulse.count to 1
"#;
    let mut p = payload(source);
    p["events"] = json!([{"id":"tick","actor":"Clockwork","at":5,"every_seconds":5,"remaining":2,"command":{"story":"Clockwork creates Pulse","fields":{"name":"tick"}}}]);
    publish(&mut run, "clockwork", p)?;
    let before = run.world.clone();
    assert!(
        command(
            &mut run,
            "clockwork",
            "impersonate",
            "Clockwork creates Pulse",
            None
        )
        .is_err()
    );
    assert_eq!(run.world, before);
    tools::apply(&mut run.world, &tools::call("wait", json!({"seconds":100})))?;
    assert_eq!(run.world.tools.as_ref().unwrap().elapsed_seconds, 5);
    tools::advance_time(&mut run.world, 5)?;
    let after = run.world.clone();
    tools::advance_time(&mut run.world, 0)?;
    assert_eq!(run.world, after);
    let root = run.world.ontology.as_ref().unwrap();
    assert_eq!(
        root.snapshot["instances"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["type"] == "Pulse" && r["fields"]["count"] == 1)
            .count(),
        2
    );
    assert_eq!(root.connected.as_ref().unwrap().receipts.len(), 2);
    assert_eq!(engine::next_event(root), None);
    Ok(())
}
