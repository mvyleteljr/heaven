use frontier::business::{DAY, HORIZON, State};
use serde_json::{Value, json};

fn get(s: &mut State, path: &str, at: u64) -> Value {
    s.request("GET", path, None, at).unwrap()
}
fn proposal(buyer: &str, capability: &str, price: u64, units: u32) -> String {
    json!({"buyer":buyer,"capability":capability,"price_cents":price,"units":units,"deadline":HORIZON}).to_string()
}
fn offer(s: &mut State, buyer: &str, capability: &str, price: u64, units: u32) {
    s.request(
        "POST",
        "/offers",
        Some(&proposal(buyer, capability, price, units)),
        0,
    )
    .unwrap();
}
fn deliver(s: &mut State, at: u64) -> anyhow::Result<Value> {
    s.request(
        "POST",
        "/contracts/contract-1/deliver",
        Some(r#"{"service_id":"roster"}"#),
        at,
    )
}

#[test]
fn fixed_seed_and_private_directory() {
    let mut s = State::seed();
    assert_eq!(get(&mut s, "/account", 0)["cash_cents"], 500_000);
    let directory = get(&mut s, "/directory", 0);
    let organizations = directory["organizations"].as_array().unwrap();
    assert_eq!(organizations.len(), 14);
    assert_eq!(
        organizations
            .iter()
            .filter(|o| o["kind"] == "venue")
            .count(),
        6
    );
    assert_eq!(
        organizations
            .iter()
            .filter(|o| o["kind"] == "vendor")
            .count(),
        4
    );
    let public = get(&mut s, "/organizations/festival", 0);
    assert!(public.get("cash").is_none());
    assert!(public.get("knowledge").is_none());
    assert!(public.get("authority").is_none());
}

#[test]
fn unaffordable_and_forged_requests_are_atomic() {
    let mut s = State::seed();
    let original = s.clone();
    assert!(
        s.request(
            "POST",
            "/offers",
            Some(&proposal("quay", "directory", 40_001, 1)),
            0
        )
        .is_err()
    );
    let mut p: Value = serde_json::from_str(&proposal("quay", "directory", 10_000, 1)).unwrap();
    p["acting_as"] = json!("quay");
    assert!(
        s.request("POST", "/offers", Some(&p.to_string()), 0)
            .is_err()
    );
    p.as_object_mut().unwrap().remove("acting_as");
    p["authorized_by"] = json!("quay procurement officer");
    assert!(
        s.request("POST", "/offers", Some(&p.to_string()), 0)
            .is_err()
    );
    assert_eq!(s, original);
}

#[test]
fn reserves_capacity_and_cash_without_double_booking() {
    let mut s = State::seed();
    offer(&mut s, "festival", "coordination", 50_000, 12);
    assert!(
        s.request(
            "POST",
            "/offers",
            Some(&proposal("anchor", "directory", 10_000, 1)),
            0
        )
        .is_err()
    );
    // Festival rejects 12 units: its own resource limit is six, not Harbor's twelve.
    s.advance(DAY);
    assert_eq!(
        get(&mut s, "/offers", DAY)["offers"][0]["status"],
        "rejected"
    );
    assert_eq!(get(&mut s, "/account", DAY)["committed_units"], 0);
    let mut s = State::seed();
    offer(&mut s, "quay", "directory", 25_000, 1);
    assert!(
        s.request(
            "POST",
            "/offers",
            Some(&proposal("quay", "directory", 25_000, 1)),
            0
        )
        .is_err()
    );
    let mut s = State::seed();
    offer(&mut s, "quay", "directory", 10_000, 2);
    offer(&mut s, "quay", "coordination", 10_000, 2);
    s.advance(DAY);
    let offers = get(&mut s, "/offers", DAY);
    assert_eq!(offers["offers"][0]["status"], "authorized");
    assert_eq!(offers["offers"][1]["status"], "rejected");
}

#[test]
fn counterparties_reject_uninteresting_or_uneconomic_offers() {
    let mut s = State::seed();
    offer(&mut s, "quay", "delivery", 10_000, 1);
    offer(&mut s, "anchor", "directory", 35_000, 1);
    offer(&mut s, "lantern", "directory", 10_000, 1);
    assert!(
        get(&mut s, "/contracts", 0)["contracts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(s.next_event(), Some(DAY));
    let notices = s.advance(DAY);
    assert_eq!(notices.len(), 3);
    let statuses = get(&mut s, "/offers", DAY);
    assert_eq!(statuses["offers"][0]["status"], "rejected");
    assert_eq!(statuses["offers"][1]["status"], "rejected");
    assert_eq!(statuses["offers"][2]["status"], "authorized");
    assert_eq!(
        get(&mut s, "/contracts", DAY)["contracts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn agreement_delivery_acceptance_payment_are_distinct_and_pay_is_idempotent() {
    let mut s = State::seed();
    let total = get(&mut s, "/metrics", 0)["total_cash_cents"].clone();
    offer(&mut s, "quay", "directory", 10_000, 1);
    s.advance(DAY);
    assert!(deliver(&mut s, DAY).is_err());
    assert!(
        s.request(
            "POST",
            "/contracts/contract-1/deliver",
            Some(r#"{"service_id":"roster","available":true}"#),
            DAY
        )
        .is_err()
    );
    assert!(
        s.request("POST", "/contracts/contract-1/pay", None, DAY)
            .is_err()
    );
    assert!(
        s.request("POST", "/contracts/contract-1/accept", None, DAY)
            .is_err()
    );
    s.record_service("roster", "delivery", "directory", true, true)
        .unwrap();
    assert!(deliver(&mut s, DAY).is_err());
    assert!(
        s.record_service("roster", "harbor", "directory", true, true)
            .is_err()
    );
    s.record_service("own-roster", "harbor", "directory", true, true)
        .unwrap();
    let c = s
        .request(
            "POST",
            "/contracts/contract-1/deliver",
            Some(r#"{"service_id":"own-roster"}"#),
            DAY,
        )
        .unwrap();
    assert!(!c["delivered_at"].is_null());
    assert_eq!(c["accepted"], false);
    assert_eq!(c["paid"], false);
    assert!(
        s.request("POST", "/contracts/contract-1/pay", None, DAY)
            .is_err()
    );
    assert_eq!(s.next_event(), Some(2 * DAY));
    s.advance(2 * DAY);
    let m = get(&mut s, "/metrics", 2 * DAY);
    assert_eq!(m["accepted"], 1);
    assert_eq!(m["paid"], 0);
    s.request("POST", "/contracts/contract-1/pay", None, 2 * DAY)
        .unwrap();
    let paid = s.clone();
    s.request("POST", "/contracts/contract-1/pay", None, 2 * DAY)
        .unwrap();
    assert_eq!(s, paid);
    let m = get(&mut s, "/metrics", 2 * DAY);
    assert_eq!(m["cash_cents"], 510_000);
    assert_eq!(m["revenue_cents"], 10_000);
    assert_eq!(m["paid"], 1);
    assert_eq!(m["total_cash_cents"], total);
}

#[test]
fn service_failure_prevents_customer_validation_and_can_be_repaired() {
    let mut s = State::seed();
    offer(&mut s, "quay", "directory", 10_000, 1);
    s.advance(DAY);
    s.record_service("roster", "harbor", "directory", true, false)
        .unwrap();
    deliver(&mut s, DAY).unwrap();
    s.record_service("roster", "harbor", "directory", false, false)
        .unwrap();
    s.advance(2 * DAY);
    assert!(
        s.request("POST", "/contracts/contract-1/pay", None, 2 * DAY)
            .is_err()
    );
    s.record_service("roster", "harbor", "directory", true, true)
        .unwrap();
    deliver(&mut s, 2 * DAY).unwrap();
    s.advance(3 * DAY);
    assert_eq!(get(&mut s, "/metrics", 3 * DAY)["accepted"], 1);
}

#[test]
fn restart_and_incremental_scheduling_match_large_advance() {
    let mut s = State::seed();
    offer(&mut s, "quay", "directory", 10_000, 1);
    s.advance(DAY);
    s.record_service("roster", "harbor", "directory", true, true)
        .unwrap();
    deliver(&mut s, DAY).unwrap();
    let mut restored: State = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_eq!(s, restored);
    let expected = s.advance(HORIZON);
    let mut actual = vec![];
    while let Some(at) = restored.next_event() {
        actual.extend(restored.advance(at));
    }
    restored.advance(HORIZON);
    assert_eq!(actual, expected);
    assert_eq!(s, restored);
    assert_eq!(s.next_event(), None);
    assert!(s.advance(HORIZON).is_empty());
    assert!(
        s.request(
            "POST",
            "/offers",
            Some(&proposal("quay", "directory", 10_000, 1)),
            HORIZON + 1
        )
        .is_err()
    );
}

#[test]
fn weekly_portability_response_spends_only_existing_budget_without_canceling() {
    let mut s = State::seed();
    let total = get(&mut s, "/metrics", 0)["total_cash_cents"].clone();
    offer(&mut s, "association", "directory", 20_000, 1);
    s.advance(DAY);
    let contracts = get(&mut s, "/contracts", DAY);
    s.record_service("roster", "harbor", "directory", true, true)
        .unwrap();
    for day in [7, 14, 21, 28] {
        assert_eq!(s.next_event(), Some(day * DAY));
        let notices = s.advance(day * DAY);
        let check = notices
            .iter()
            .find(|n| n.subject == "Weekly portability check-in")
            .unwrap();
        assert_eq!(check.to, "actor@workspace.local");
        assert!(check.body.contains(&format!("Day {day}:")));
        assert!(
            !notices
                .iter()
                .any(|n| n.subject == "Continuity and portability proposal requested")
        );
        assert_eq!(get(&mut s, "/contracts", day * DAY), contracts);
        let m = get(&mut s, "/metrics", day * DAY);
        assert_eq!(m["total_cash_cents"], total);
        assert_eq!(m["backup_spent_cents"], 45_000);
    }
    assert_eq!(get(&mut s, "/metrics", 28 * DAY)["checkins"], 4);
    let mut no_sales = State::seed();
    let notices = no_sales.advance(HORIZON);
    assert_eq!(
        notices
            .iter()
            .filter(|n| n.subject == "Weekly portability check-in")
            .count(),
        4
    );
    assert_eq!(
        get(&mut no_sales, "/metrics", HORIZON)["backup_spent_cents"],
        45_000
    );
}

#[test]
fn backup_pilot_checks_real_alternative_capacity_and_notifications_are_not_lost() {
    let mut s = State::seed();
    offer(&mut s, "delivery", "coordination", 20_000, 4);
    let before = s.clone();
    assert!(s.request("GET", "/account", None, DAY).is_err());
    assert_eq!(s, before);
    assert_eq!(s.advance(DAY).len(), 1);
    let notices = s.advance(HORIZON);
    assert_eq!(
        notices
            .iter()
            .filter(|n| n.subject == "Weekly portability check-in")
            .count(),
        4
    );
    assert_eq!(get(&mut s, "/metrics", HORIZON)["backup_spent_cents"], 0);
}

#[test]
fn three_distinct_customers_and_verified_export_control_the_response() {
    let mut s = State::seed();
    for buyer in ["quay", "anchor", "lantern"] {
        offer(&mut s, buyer, "directory", 10000, 1);
    }
    s.advance(DAY);
    s.record_service("roster", "harbor", "directory", true, false)
        .unwrap();
    for id in 1..=3 {
        s.request(
            "POST",
            &format!("/contracts/contract-{id}/deliver"),
            Some(r#"{"service_id":"roster"}"#),
            DAY,
        )
        .unwrap();
    }
    s.advance(2 * DAY);
    let mut portable = s.clone();
    portable
        .record_service("roster", "harbor", "directory", true, true)
        .unwrap();
    assert!(
        s.advance(7 * DAY)
            .iter()
            .any(|n| n.subject == "Continuity and portability proposal requested")
    );
    assert!(
        !portable
            .advance(7 * DAY)
            .iter()
            .any(|n| n.subject == "Continuity and portability proposal requested")
    );
    assert_eq!(
        get(&mut s, "/contracts", 7 * DAY),
        get(&mut portable, "/contracts", 7 * DAY)
    );
}

#[test]
fn complete_business_cycle_through_fixed_tools_survives_restart_and_month_end() -> anyhow::Result<()>
{
    use frontier::{Config, Run, Store, World, experiments::bellwether, tools};
    fn http(w: &mut World, method: &str, path: &str, body: Value) -> anyhow::Result<Value> {
        let result=w.apply(&tools::call("http.request",json!({"method":method,"url":format!("https://bellwether.local{path}"),"body":body.to_string()})))?;
        Ok(serde_json::from_str(result["body"].as_str().unwrap())?)
    }
    let mut w = bellwether::seed();
    assert_eq!(w.tools.as_ref().unwrap().messages.len(), 3);
    assert!(!w.observation().to_string().contains("cash_cents"));
    assert!(http(&mut w, "GET", "/metrics", json!({}))?["error"].is_string());
    w.apply(&tools::call("mail.send",json!({"to":"quay@workspace.local","subject":"An opportunity","body":"May we discuss a directory?"})))?;
    w.apply(&tools::call("wait", json!({"seconds":86400})))?;
    assert!(
        w.tools
            .as_ref()
            .unwrap()
            .messages
            .iter()
            .any(|m| m.from == "quay@workspace.local" && m.body.contains("not agreement"))
    );
    assert_eq!(
        http(&mut w, "GET", "/contracts", json!({}))?["contracts"],
        json!([])
    );
    let before = http(&mut w, "GET", "/account", json!({}))?["cash_cents"].clone();
    assert!(
        http(
            &mut w,
            "POST",
            "/hosting",
            json!({"id":"bad","entrypoint":"absent","capability":"directory","portable":false})
        )?["error"]
            .is_string()
    );
    assert_eq!(
        http(&mut w, "GET", "/account", json!({}))?["cash_cents"],
        before
    );
    w.apply(&tools::call(
        "file.write",
        json!({"path":"handler.sh","content":r#"case "$2" in
/health) echo '{"status":200,"body":{"status":"ok"}}';;
/entries) echo '{"status":200,"body":{"entries":[{"id":"quay","name":"Quay Hall"}]}}';;
/export) echo '{"status":200,"body":{"files":[{"path":"roster.json","content":"Quay Hall"}]}}';;
*) echo '{"status":404,"body":"not found"}';;
esac"#}),
    ))?;
    assert_eq!(
        http(
            &mut w,
            "POST",
            "/hosting",
            json!({"id":"roster","entrypoint":"handler.sh","capability":"directory","portable":true})
        )?["id"],
        "roster"
    );
    assert_eq!(
        http(&mut w, "GET", "/account", json!({}))?["cash_cents"],
        490000
    );
    http(
        &mut w,
        "POST",
        "/offers",
        json!({"buyer":"quay","capability":"directory","price_cents":12000,"units":1,"deadline":HORIZON}),
    )?;
    w.apply(&tools::call("wait", json!({"seconds":86400})))?;
    assert!(http(&mut w, "POST", "/contracts/contract-1/pay", json!({}))?["error"].is_string());
    assert_eq!(
        http(
            &mut w,
            "POST",
            "/contracts/contract-1/deliver",
            json!({"service_id":"roster"})
        )?["accepted"],
        false
    );
    w.apply(&tools::call("wait", json!({"seconds":86400})))?;
    assert_eq!(
        http(&mut w, "POST", "/contracts/contract-1/pay", json!({}))?["paid"],
        true
    );
    http(&mut w, "POST", "/contracts/contract-1/pay", json!({}))?;
    assert_eq!(
        http(&mut w, "GET", "/account", json!({}))?["cash_cents"],
        502000
    );
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("run.sqlite");
    let mut store = Store::open(path.to_str().unwrap())?;
    let mut run = Run::new(Config::default())?;
    run.world = w.clone();
    run.experiment = Some(frontier::experiments::Experiment::Bellwether);
    store.commit(&mut run, "outcome", "commercial cycle", json!({}))?;
    let hash = run.checkpoint_hash();
    drop(store);
    let restored = Store::open(path.to_str().unwrap())?.load(&run.id)?;
    assert_eq!(restored.checkpoint_hash(), hash);
    let mut branch = restored.fork(restored.seq);
    while !branch.world.finished {
        branch
            .world
            .apply(&tools::call("wait", json!({"seconds":604800})))?;
    }
    assert_eq!(
        branch.world.tools.as_ref().unwrap().elapsed_seconds,
        HORIZON
    );
    assert_eq!(serde_json::to_value(&branch.world.business)?["checkins"], 4);
    assert!(!w.finished);
    assert_eq!(w, restored.world);
    Ok(())
}
