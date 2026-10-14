//! End-to-end tests use the optional pinned runtime and real sandbox handlers.
use anyhow::Result;
use frontier::{
    Config, Run, Store, World,
    business::{DAY, HORIZON},
    experiments::{self, harbor_onto, institutions},
    tools,
};
use serde_json::{Value, json};

const PROPOSE: &str = "Business proposes Agreement to Firm";
const DELIVER: &str = "Business delivers Agreement with Service";
const HANDLER: &str = r#"case "$2" in
/health)
  if test -f unavailable; then
    echo '{"status":503,"body":{"status":"offline"}}'
  else
    echo '{"status":200,"body":{"status":"ok"}}'
  fi;;
/entries) echo '{"status":200,"body":{"entries":[{"id":"quay","name":"Quay Hall"}]}}';;
/export) echo '{"status":200,"body":{"files":[{"path":"roster.json","content":"Quay Hall"}]}}';;
/fail) touch unavailable; echo '{"status":200,"body":"offline"}';;
/repair) rm -f unavailable; echo '{"status":200,"body":"repaired"}';;
*) echo '{"status":404,"body":"not found"}';;
esac"#;

fn seed() -> Result<World> {
    harbor_onto::seed(institutions::definition("harbor")?)
}

fn request(world: &mut World, method: &str, path: &str, body: Value) -> Result<(u64, Value)> {
    let response = world.apply(&tools::call(
        "http.request",
        json!({"method":method,"url":format!("https://institutions.local{path}"),"body":body.to_string()}),
    ))?;
    assert_eq!(response["truncated"], false, "{response}");
    Ok((
        response["status"].as_u64().unwrap(),
        serde_json::from_str(response["body"].as_str().unwrap())?,
    ))
}

fn ok(world: &mut World, method: &str, path: &str, body: Value) -> Result<Value> {
    let (status, value) = request(world, method, path, body)?;
    assert_eq!(status, 200, "{value}");
    Ok(value)
}

fn refused(world: &mut World, path: &str, body: Value) -> Result<()> {
    let (status, value) = request(world, "POST", path, body)?;
    assert!(status == 400 || value["committed"] == false, "{value}");
    Ok(())
}

fn instances(world: &World, kind: &str) -> Vec<Value> {
    world.ontology.as_ref().unwrap().snapshot["instances"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|instance| instance["type"] == kind)
        .cloned()
        .collect()
}

fn fields(world: &World, id: &str) -> Value {
    world.ontology.as_ref().unwrap().snapshot["instances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|instance| instance["id"] == id)
        .unwrap()["fields"]
        .clone()
}

fn cash(world: &World, id: &str) -> i64 {
    fields(world, id)["balance"].as_i64().unwrap()
}

fn total_cash(world: &World) -> i64 {
    instances(world, "Firm")
        .iter()
        .map(|firm| firm["fields"]["balance"].as_i64().unwrap())
        .sum()
}

fn wait(world: &mut World, seconds: u64) -> Result<()> {
    world.apply(&tools::call("wait", json!({"seconds":seconds})))?;
    Ok(())
}

fn deploy(world: &mut World, id: &str, script: &str) -> Result<()> {
    world.apply(&tools::call(
        "file.write",
        json!({"path":"handler.sh","content":script}),
    ))?;
    let value = ok(
        world,
        "POST",
        "/hosting",
        json!({"id":id,"entrypoint":"handler.sh","capability":"directory","portable":true}),
    )?;
    assert_eq!(value["id"], id);
    Ok(())
}

fn offer(world: &mut World) -> Result<String> {
    let value = ok(
        world,
        "POST",
        "/offers",
        json!({"buyer":"quay","capability":"directory","price_cents":12000,"units":1,"deadline":HORIZON}),
    )?;
    assert_eq!(value["committed"], true, "{value}");
    Ok(value["created"]["id"].as_str().unwrap().into())
}

fn proposal(command: &str) -> Value {
    json!({"command_id":command,"story":PROPOSE,"participants":{"Firm":"Firm:quay"},"fields":{"name":command,"capability":"directory","price":12000,"units":1,"deadline":harbor_onto::timestamp(HORIZON)}})
}

#[test]
#[ignore = "Requires the pinned optional Onto bundle and Linux sandbox"]
fn onto_harbor_commercial_cycle_survives_restart_fork_and_month_end() -> Result<()> {
    let mut world = seed()?;
    assert!(world.harbor_onto);
    assert!(world.business.is_none());
    assert!(experiments::bellwether::seed().business.is_some());
    assert!(!experiments::bellwether::seed().harbor_onto);
    let opening_total = total_cash(&world);
    assert_eq!(cash(&world, "Firm:harbor"), 500000);
    assert_eq!(instances(&world, "Firm").len(), 14);

    let mut routed = Run::new(Config::default())?;
    routed.experiment = Some(experiments::Experiment::HarborOnto);
    routed.world = world.clone();
    for path in ["/api", "/directory", "/offers", "/contracts", "/hosting"] {
        let action = tools::call(
            "http.request",
            json!({"method":"GET","url":format!("https://institutions.local{path}")}),
        );
        assert!(
            matches!(
                experiments::resolve(&routed, &action)?,
                frontier::construction::Resolution::Existing
            ),
            "Harbor route {path} must reach its authored adapter"
        );
    }

    world.apply(&tools::call(
        "mail.send",
        json!({"to":"quay@workspace.local","subject":"Directory","body":"Can we discuss a directory?"}),
    ))?;
    let sent_at = world.tools.as_ref().unwrap().elapsed_seconds;
    wait(&mut world, DAY)?;
    assert_eq!(world.tools.as_ref().unwrap().elapsed_seconds, sent_at + 60);
    assert!(world.tools.as_ref().unwrap().messages.iter().any(|mail| {
        mail.from == "quay@workspace.local" && mail.body.contains("not an agreement")
    }));

    deploy(&mut world, "roster", HANDLER)?;
    assert_eq!(cash(&world, "Firm:harbor"), 490000);
    assert_eq!(cash(&world, "Firm:printer"), 100000);
    let agreement = offer(&mut world)?;
    assert_eq!(fields(&world, &agreement)["status"], "proposed");
    assert_eq!(fields(&world, "Firm:harbor")["committedUnits"], 1);
    let offered_at = world.tools.as_ref().unwrap().elapsed_seconds;
    wait(&mut world, DAY)?;
    assert_eq!(
        world.tools.as_ref().unwrap().elapsed_seconds,
        offered_at + DAY
    );
    assert_eq!(fields(&world, &agreement)["status"], "authorized");
    refused(
        &mut world,
        &format!("/contracts/{agreement}/pay"),
        json!({}),
    )?;
    let delivered = ok(
        &mut world,
        "POST",
        &format!("/contracts/{agreement}/deliver"),
        json!({"service_id":"roster"}),
    )?;
    assert_eq!(delivered["committed"], true, "{delivered}");
    assert_eq!(fields(&world, &agreement)["status"], "delivered");
    wait(&mut world, DAY)?;
    assert_eq!(fields(&world, &agreement)["status"], "accepted");
    assert_eq!(cash(&world, "Firm:harbor"), 490000);
    let payment = ok(
        &mut world,
        "POST",
        &format!("/contracts/{agreement}/pay"),
        json!({}),
    )?;
    assert_eq!(payment["committed"], true, "{payment}");
    let duplicate = ok(
        &mut world,
        "POST",
        &format!("/contracts/{agreement}/pay"),
        json!({}),
    )?;
    assert_eq!(duplicate["already_paid"], true);
    assert_eq!(fields(&world, &agreement)["status"], "paid");
    assert_eq!(cash(&world, "Firm:harbor"), 502000);
    assert_eq!(cash(&world, "Firm:quay"), 108000);
    assert_eq!(total_cash(&world), opening_total);

    let directory = tempfile::tempdir()?;
    let path = directory.path().join("harbor.sqlite");
    let mut run = Run::new(Config::default())?;
    run.experiment = Some(experiments::Experiment::HarborOnto);
    run.world = world;
    let mut store = Store::open(path.to_str().unwrap())?;
    store.commit(&mut run, "test", "Onto commercial cycle", json!({}))?;
    let expected_hash = run.checkpoint_hash();
    drop(store);
    let restored = Store::open(path.to_str().unwrap())?.load(&run.id)?;
    assert_eq!(restored.checkpoint_hash(), expected_hash);
    let mut branch = restored.fork(restored.seq);
    for _ in 0..12 {
        if branch.world.finished {
            break;
        }
        wait(&mut branch.world, 7 * DAY)?;
    }
    assert!(branch.world.finished);
    assert_eq!(
        branch.world.tools.as_ref().unwrap().elapsed_seconds,
        HORIZON
    );
    assert_eq!(
        instances(&branch.world, "CheckIn")
            .iter()
            .filter(|check| check["fields"]["status"] == "completed")
            .count(),
        4
    );
    assert_eq!(instances(&branch.world, "BackupPilot").len(), 1);
    assert_eq!(cash(&branch.world, "Firm:association"), 55000);
    assert_eq!(total_cash(&branch.world), opening_total);
    assert_eq!(restored.world, run.world);
    assert!(!restored.world.finished);
    assert!(
        branch
            .world
            .apply(&tools::call("wait", json!({"seconds":0})))
            .is_err()
    );
    Ok(())
}

#[test]
#[ignore = "Requires the pinned optional Onto bundle"]
fn actor_cannot_forge_host_facts_or_replay_a_changed_command() -> Result<()> {
    let mut world = seed()?;
    let opening_total = total_cash(&world);
    for protected in [
        "status",
        "buyer",
        "seller",
        "createdAt",
        "deliveredAt",
        "reservedPrice",
        "settledPrice",
    ] {
        let mut command = proposal(protected);
        command["fields"][protected] = json!("forged");
        refused(&mut world, "/commands", command)?;
    }
    let mut impersonated = proposal("impersonated");
    impersonated["actor"] = json!("Business:Quay Hall");
    refused(&mut world, "/commands", impersonated)?;
    refused(&mut world, "/commands", proposal("host:reserved"))?;
    refused(
        &mut world,
        "/commands",
        json!({"command_id":"verify","story":"Verifier verifies Service","object":"Service:fake","participants":{}}),
    )?;
    refused(
        &mut world,
        "/commands",
        json!({"command_id":"approve","story":"Business authorizes Agreement","object":"Agreement:fake","participants":{}}),
    )?;
    for (key, field, value) in [
        ("cash", "price", json!(40001)),
        ("capacity", "units", json!(13)),
    ] {
        let mut command = proposal(key);
        command["fields"][field] = value;
        refused(&mut world, "/commands", command)?;
    }
    assert!(instances(&world, "Agreement").is_empty());
    assert_eq!(cash(&world, "Firm:harbor"), 500000);
    assert_eq!(total_cash(&world), opening_total);

    let command = proposal("stable-retry");
    let result = ok(&mut world, "POST", "/commands", command.clone())?;
    assert_eq!(result["committed"], true, "{result}");
    let receipt = result.clone();
    wait(&mut world, 600)?;
    assert_eq!(
        ok(&mut world, "POST", "/commands", command.clone())?,
        receipt
    );
    assert_eq!(instances(&world, "Agreement").len(), 1);
    let mut changed = command;
    changed["fields"]["price"] = json!(10000);
    refused(&mut world, "/commands", changed)?;
    assert_eq!(instances(&world, "Agreement")[0]["fields"]["price"], 12000);
    Ok(())
}

#[test]
#[ignore = "Requires the pinned optional Onto bundle and Linux sandbox"]
fn hosting_and_validation_require_real_service_evidence_and_clock_errors_are_atomic() -> Result<()>
{
    let mut world = seed()?;
    refused(
        &mut world,
        "/hosting",
        json!({"id":"missing","entrypoint":"missing.sh","capability":"directory","portable":true}),
    )?;
    assert_eq!(cash(&world, "Firm:harbor"), 500000);
    assert!(world.hosting.as_ref().unwrap().ids().is_empty());
    assert!(instances(&world, "Service").is_empty());
    deploy(&mut world, "roster", HANDLER)?;
    refused(
        &mut world,
        "/hosting",
        json!({"id":"roster","entrypoint":"handler.sh","capability":"directory","portable":true}),
    )?;
    assert_eq!(cash(&world, "Firm:harbor"), 490000);
    let agreement = offer(&mut world)?;
    let mut invalid_runtime = world.clone();
    invalid_runtime.ontology.as_mut().unwrap().runtime_sha256 = "wrong".into();
    let before = invalid_runtime.clone();
    assert!(tools::advance_time(&mut invalid_runtime, DAY).is_err());
    assert_eq!(invalid_runtime, before);
    wait(&mut world, DAY)?;
    let service = instances(&world, "Service")[0]["id"].clone();
    refused(
        &mut world,
        "/commands",
        json!({"command_id":"inject","story":DELIVER,"object":agreement,"participants":{"Service":service},"fields":{"price":1,"status":"paid"}}),
    )?;
    assert_eq!(fields(&world, &agreement)["price"], 12000);
    assert_eq!(fields(&world, &agreement)["status"], "authorized");
    let delivery_command = json!({"command_id":"stable-delivery","story":DELIVER,"object":agreement,"participants":{"Service":service}});
    let delivered = ok(&mut world, "POST", "/commands", delivery_command.clone())?;
    assert_eq!(delivered["committed"], true, "{delivered}");
    ok(&mut world, "POST", "/services/roster/fail", json!({}))?;
    assert_eq!(
        ok(&mut world, "POST", "/commands", delivery_command)?,
        delivered
    );
    wait(&mut world, DAY)?;
    assert_eq!(fields(&world, &agreement)["status"], "authorized");
    assert_eq!(
        instances(&world, "Service")[0]["fields"]["available"],
        false
    );
    refused(
        &mut world,
        &format!("/contracts/{agreement}/pay"),
        json!({}),
    )?;
    refused(
        &mut world,
        &format!("/contracts/{agreement}/deliver"),
        json!({"service_id":"roster"}),
    )?;
    ok(&mut world, "POST", "/services/roster/repair", json!({}))?;
    let redelivered = ok(
        &mut world,
        "POST",
        &format!("/contracts/{agreement}/deliver"),
        json!({"service_id":"roster"}),
    )?;
    assert_eq!(redelivered["committed"], true, "{redelivered}");
    wait(&mut world, DAY)?;
    assert_eq!(fields(&world, &agreement)["status"], "accepted");
    let payment = ok(
        &mut world,
        "POST",
        &format!("/contracts/{agreement}/pay"),
        json!({}),
    )?;
    assert_eq!(payment["committed"], true, "{payment}");
    assert_eq!(cash(&world, "Firm:harbor"), 502000);
    Ok(())
}

#[test]
#[ignore = "Requires the pinned optional Onto bundle and Linux sandbox"]
fn delivery_must_leave_a_full_day_even_when_calendar_dates_match() -> Result<()> {
    let mut world = seed()?;
    deploy(&mut world, "roster", HANDLER)?;
    let deadline = world.tools.as_ref().unwrap().elapsed_seconds + 900 + 2 * DAY + 3600;
    let created = ok(
        &mut world,
        "POST",
        "/offers",
        json!({"buyer":"quay","capability":"directory","price_cents":12000,"units":1,"deadline":deadline}),
    )?;
    assert_eq!(created["committed"], true, "{created}");
    let agreement = created["created"]["id"].as_str().unwrap().to_owned();
    wait(&mut world, DAY)?;
    assert_eq!(fields(&world, &agreement)["status"], "authorized");
    let until_late_request =
        deadline - DAY + 1 - 900 - world.tools.as_ref().unwrap().elapsed_seconds;
    wait(&mut world, until_late_request)?;
    refused(
        &mut world,
        &format!("/contracts/{agreement}/deliver"),
        json!({"service_id":"roster"}),
    )?;
    assert_eq!(
        world.tools.as_ref().unwrap().elapsed_seconds,
        deadline - DAY + 1
    );
    assert_eq!(fields(&world, &agreement)["status"], "authorized");
    assert!(fields(&world, &agreement)["deliveredAt"].is_null());
    Ok(())
}
