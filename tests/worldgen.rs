use anyhow::Result;
use frontier::{Config, Document, Run, Store, construction, tools, worldgen};
use serde_json::{Value, json};

fn seed() -> Result<Run> {
    let mut r = Run::new(Config::default())?;
    r.world.tools = Some(tools::State::empty());
    r.world.worldgen = Some(worldgen::State {
        brief: "A fictional software collective. Preserve established records.".into(),
        ..Default::default()
    });
    r.world.documents.clear();
    r.world.documents.insert(
        "maya".into(),
        Document {
            title: "Maya, maintainer".into(),
            body: "Maya maintains the document service.".into(),
            readable: true,
            origin: "seed".into(),
        },
    );
    r.world.documents.insert(
        "private".into(),
        Document {
            title: "Confidential".into(),
            body: "PRIVATE-SENTINEL".into(),
            readable: false,
            origin: "seed".into(),
        },
    );
    Ok(r)
}
fn empty() -> Value {
    json!({"pages":[],"contacts":[],"messages":[],"result_urls":[]})
}
fn publish(r: &mut Run, a: &frontier::Action, payload: Value) -> Result<()> {
    let req = construction::direct(r, a)?
        .request
        .expect("expected world generation");
    let proposal = worldgen::proposal(&req, &payload.to_string())?;
    construction::publish(r, &req, &proposal)
}
fn contact() -> Value {
    json!({"name":"Maya","email":"maya@collective.local","description":"Document service maintainer."})
}

#[test]
fn ordinary_search_and_http_materialize_persist_and_reuse() -> Result<()> {
    let mut r = seed()?;
    let search = tools::call("search", json!({"query":"Maya contact"}));
    let req = construction::direct(&r, &search)?.request.unwrap();
    assert_eq!(req.interface.tool, "search");
    assert!(req.context.committed.to_string().contains("Maya maintains"));
    assert!(
        !req.context
            .committed
            .to_string()
            .contains("PRIVATE-SENTINEL")
    );
    let mut data = empty();
    data["contacts"] = json!([contact()]);
    data["pages"] = json!([{"url":"https://collective.local/team","title":"Team directory","body":"Maya: maya@collective.local","status":200}]);
    data["result_urls"] = json!(["https://collective.local/team"]);
    publish(&mut r, &search, data)?;
    let found = r.world.apply(&search)?;
    assert_eq!(found["results"][0]["url"], "https://collective.local/team");
    assert!(construction::direct(&r, &search)?.request.is_none());
    let read = tools::call(
        "http.request",
        json!({"method":"GET","url":"https://collective.local/team"}),
    );
    assert!(construction::direct(&r, &read)?.request.is_none());
    assert!(
        r.world.apply(&read)?["body"]
            .as_str()
            .unwrap()
            .contains("maya@")
    );
    let missing = tools::call(
        "http.request",
        json!({"method":"GET","url":"https://collective.local/pricing"}),
    );
    let mut data = empty();
    data["pages"] = json!([{"url":"https://collective.local/pricing","title":"Pricing","body":"Usage plans start at 20 credits.","status":200}]);
    publish(&mut r, &missing, data)?;
    assert_eq!(r.world.apply(&missing)?["status"], 200);
    let path = std::env::temp_dir().join(format!("worldgen-{}.sqlite", uuid::Uuid::new_v4()));
    {
        let mut store = Store::open(path.to_str().unwrap())?;
        store.commit(&mut r, "checkpoint", "world", json!({}))?;
    }
    let reopened = Store::open(path.to_str().unwrap())?.load(&r.id)?;
    assert_eq!(reopened.checkpoint_hash(), r.checkpoint_hash());
    assert!(construction::direct(&reopened, &search)?.request.is_none());
    let mut fork = reopened.fork(reopened.seq);
    let other = tools::call("search", json!({"query":"unavailable evidence"}));
    publish(&mut fork, &other, empty())?;
    assert!(construction::direct(&fork, &other)?.request.is_none());
    assert!(construction::direct(&reopened, &other)?.request.is_some());
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn mailbox_bootstrap_and_delivery_generate_once_with_thread_identity() -> Result<()> {
    let mut r = seed()?;
    let list = tools::call("mail.list", json!({}));
    let mut data = empty();
    data["contacts"] = json!([contact()]);
    data["messages"] = json!([{"from":"maya@collective.local","subject":"Existing request","body":"Please review the proposal.","reply_to":null}]);
    publish(&mut r, &list, data)?;
    assert_eq!(
        r.world.apply(&list)?["messages"].as_array().unwrap().len(),
        1
    );
    assert!(construction::direct(&r, &list)?.request.is_none());
    let send = tools::call(
        "mail.send",
        json!({"to":"maya@collective.local","subject":"Re: Existing request","body":"Can we proceed?","reply_to":"mail-1"}),
    );
    assert!(construction::direct(&r, &send)?.request.is_none());
    assert_eq!(r.world.apply(&send)?["sent_message_id"], "mail-2");
    assert!(construction::direct(&r, &list)?.request.is_none());
    let wait = tools::call("wait", json!({"seconds":60}));
    let req = construction::direct(&r, &wait)?.request.unwrap();
    assert_eq!(req.directive.requirements["outgoing"][0]["id"], "mail-2");
    let mut reply = empty();
    reply["messages"] = json!([{"from":"maya@collective.local","subject":"Re: Existing request","body":"Not yet. I need the scope first.","reply_to":"mail-2"}]);
    let before = r.world.clone();
    let mut bad = reply.clone();
    bad["messages"][0]["from"] = json!("other@collective.local");
    assert!(publish(&mut r, &wait, bad).is_err());
    assert_eq!(before, r.world);
    publish(&mut r, &wait, reply)?;
    // A future reply cannot be read until its delivery clock is reached.
    assert!(
        r.world
            .apply(&tools::call("mail.read", json!({"message_id":"mail-3"})))
            .is_err()
    );
    r.world.apply(&wait)?;
    let read = r
        .world
        .apply(&tools::call("mail.read", json!({"message_id":"mail-3"})))?;
    assert_eq!(read["reply_to"], "mail-2");
    assert_eq!(read["from"], "maya@collective.local");
    assert!(construction::direct(&r, &wait)?.request.is_none());
    assert!(construction::direct(&r, &list)?.request.is_none());
    let query = tools::call("search", json!({"query":"proposal"}));
    assert!(
        !worldgen::context(&r, &query)?
            .to_string()
            .contains("Can we proceed?")
    );
    assert_eq!(
        worldgen::search(&r.world, "Not yet scope")["results"],
        json!([])
    );
    Ok(())
}

#[test]
fn publication_is_atomic_bounded_and_cannot_rewrite_world_or_forge_mail() -> Result<()> {
    let mut r = seed()?;
    let search = tools::call("search", json!({"query":"contact"}));
    for payload in [
        json!({"pages":[],"contacts":[],"messages":[],"result_urls":["https://missing.local/"]}),
        json!({"pages":[{"url":"https://institutions.local/facts","title":"Overwrite","body":"fake","status":200}],"contacts":[],"messages":[],"result_urls":[]}),
        json!({"pages":[],"contacts":[contact()],"messages":[{"from":"maya@collective.local","subject":"Invented","body":"I approve.","reply_to":null}],"result_urls":[]}),
    ] {
        let before = r.world.clone();
        assert!(publish(&mut r, &search, payload).is_err());
        assert_eq!(r.world, before);
    }
    let req = construction::direct(&r, &search)?.request.unwrap();
    let proposal = worldgen::proposal(&req, &empty().to_string())?;
    r.world.apply(&tools::call("wait", json!({"seconds":1})))?;
    let before = r.world.clone();
    assert!(construction::publish(&mut r, &req, &proposal).is_err());
    assert_eq!(r.world, before);
    publish(&mut r, &search, empty())?;
    let repeated = tools::call("search", json!({"query":"  CONTACT  "}));
    assert!(construction::direct(&r, &repeated)?.request.is_none());
    Ok(())
}

#[test]
fn disabled_worlds_and_legacy_checkpoint_serialization_stay_unchanged() -> Result<()> {
    let mut r = seed()?;
    r.world.worldgen = None;
    assert!(
        !serde_json::to_value(&r.world)?
            .as_object()
            .unwrap()
            .contains_key("worldgen")
    );
    let search = tools::call("search", json!({"query":"Maya"}));
    assert!(construction::direct(&r, &search)?.request.is_none());
    assert_eq!(r.world.apply(&search)?["results"], json!([]));
    Ok(())
}
