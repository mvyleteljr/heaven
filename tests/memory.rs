use anyhow::Result;
use frontier::{Config, Run, Store, construction, experiments, memory, tools};
use serde_json::json;

fn run() -> Result<Run> {
    let mut run = Run::new(Config {
        memory: Some(memory::Policy::default()),
        context_limit_bytes: 64_000,
        ..Config::default()
    })?;
    run.experiment = Some(experiments::Experiment::Bellwether);
    run.world = experiments::bellwether::seed();
    memory::configure(&mut run)?;
    Ok(run)
}
fn summary(text: &str, sources: Vec<usize>) -> memory::Summary {
    memory::Summary {
        items: vec![memory::Item {
            kind: memory::Kind::Fact,
            text: text.into(),
            sources,
        }],
    }
}

#[test]
fn summaries_validate_supplied_citations_atomically_and_retain_exact_history() -> Result<()> {
    let mut run = run()?;
    run.world.transcript = vec![
        json!({"result":"invoice €42"}),
        json!({"result":"unpaid"}),
        json!({"result":"later"}),
    ];
    let before = serde_json::to_value(&run)?;
    for invalid in [
        summary("invented", vec![9]),
        summary("future", vec![2]),
        summary("", vec![0]),
        summary("x", vec![]),
    ] {
        assert!(memory::publish(&mut run, 2, invalid).is_err());
        assert_eq!(serde_json::to_value(&run)?, before);
    }
    memory::publish(&mut run, 2, summary("invoice remains unpaid", vec![0, 1]))?;
    assert_eq!(memory::observation(&run)["history"][0]["index"], 2);
    assert_eq!(run.world.transcript.len(), 3);
    assert!(memory::read(&run.world, 0, 1, 1).is_ok());
    let exact = serde_json::to_string(&run.world.transcript[0])?;
    let mut offset = 0;
    let mut assembled = String::new();
    while offset < exact.len() {
        let page = memory::read(&run.world, 0, offset, 4)?;
        assembled.push_str(page["content"].as_str().unwrap());
        offset = page["next_offset"].as_u64().unwrap() as usize;
    }
    assert_eq!(assembled, exact);
    let euro = exact.find('€').unwrap();
    assert!(memory::read(&run.world, 0, euro + 1, 4).is_err());
    assert!(memory::read(&run.world, 0, euro, 1).is_err());
    assert_eq!(
        memory::search(&run.world, "INVOICE", 0)?["matches"][0]["index"],
        0
    );
    memory::publish(
        &mut run,
        3,
        summary("invoice remains unpaid", vec![0, 1, 2]),
    )?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("memory.sqlite");
    let mut store = Store::open(path.to_str().unwrap())?;
    store.commit(&mut run, "memory", "test", json!({}))?;
    drop(store);
    let restored = Store::open(path.to_str().unwrap())?.load(&run.id)?;
    assert_eq!(restored.checkpoint_hash(), run.checkpoint_hash());
    assert_eq!(
        restored.fork(run.seq).checkpoint_hash(),
        run.checkpoint_hash()
    );
    Ok(())
}

#[test]
fn memory_is_opt_in_and_never_enters_generator_context() -> Result<()> {
    let legacy = Run::new(Config::default())?;
    assert!(serde_json::to_value(&legacy)?.get("memory").is_none());
    assert!(
        serde_json::to_value(&legacy.config)?
            .get("memory")
            .is_none()
    );
    let mut run = run()?;
    run.world
        .transcript
        .push(json!({"result":"PRIVATE_MEMORY_SENTINEL"}));
    memory::publish(&mut run, 1, summary("PRIVATE_MEMORY_SENTINEL", vec![0]))?;
    let action = tools::call(
        "http.request",
        json!({"method":"GET","url":"https://bellwether.local/organizations/quay/about"}),
    );
    let request = construction::direct(&run, &action)?.request.unwrap();
    assert!(!serde_json::to_string(&request)?.contains("PRIVATE_MEMORY_SENTINEL"));
    let mut old = experiments::bellwether::seed();
    assert!(
        old.apply(&tools::call("history.read", json!({"index":0})))
            .is_err()
    );
    Ok(())
}
