//! Researcher-scripted probes of the production construction/execution path.
//! The generator is live; these probes make no claim about autonomous actors.
use super::*;
use std::{collections::BTreeMap, io::Write};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Spec {
    config: Config,
    institution: frontier::ontology::Definition,
    generation: frontier::institutions::Policy,
    steps: Vec<Probe>,
    max_generation_ms: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Probe {
    label: String,
    action: Action,
    expect: Value,
    same_as: Option<String>,
    select: Option<String>,
}
fn contains(actual: &Value, expected: &Value) -> bool {
    match expected {
        Value::Object(map) => {
            actual.is_object()
                && map
                    .iter()
                    .all(|(k, v)| actual.get(k).is_some_and(|a| contains(a, v)))
        }
        Value::Array(items) => actual.as_array().is_some_and(|values| {
            items
                .iter()
                .all(|item| values.iter().any(|v| contains(v, item)))
        }),
        _ => actual == expected,
    }
}

pub async fn run(app: &App, file: &str) -> Result<()> {
    let spec_bytes = std::fs::read(file)?;
    let spec: Spec = serde_json::from_slice(&spec_bytes)?;
    spec.config.validate()?;
    ensure!(
        !spec.config.demo && app.key.is_some(),
        "Proof requires an explicit live configuration and private provider key"
    );
    ensure!(
        !spec.steps.is_empty() && spec.steps.len() <= spec.config.action_limit as usize,
        "Invalid probe/action count"
    );
    ensure!(
        spec.max_generation_ms > 0,
        "Specify a generation latency acceptance bound"
    );
    let mut seen = std::collections::BTreeSet::new();
    for probe in &spec.steps {
        ensure!(seen.insert(probe.label.clone()), "Duplicate probe label");
        tools::validate_action(&probe.action)?;
        ensure!(
            probe
                .same_as
                .as_ref()
                .is_none_or(|s| s != &probe.label && seen.contains(s)),
            "Comparison must name an earlier probe"
        );
    }
    // Resolve endpoint settings as the HTTP creation path does. An omitted
    // endpoint must never become provider.only=[""] on the wire.
    let mut config = spec.config.clone();
    validate_actor(app, &mut config).await?;
    let output = std::env::var("FRONTIER_PROOF_OUTPUT")
        .map_err(|_| anyhow::anyhow!("Set FRONTIER_PROOF_OUTPUT to a new output path"))?;
    let mut report_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    let mut state = frontier::ontology::State::initialize(spec.institution)?;
    frontier::institutions::enable(&mut state, spec.generation)?;
    let mut run = Run::new(config)?;
    run.name = "JIT executable generation / researcher probes".into();
    run.world.documents.clear();
    run.world.goal =
        "Researcher-controlled infrastructure probes, not an autonomous actor trial".into();
    run.world.tools = Some(tools::State::empty());
    run.world.ontology = Some(state);
    run.experiment = Some(experiments::Experiment::Institutions);
    let id = run.id.clone();
    app.store.lock().unwrap().commit(&mut run,"seed","Live generator proof",json!({"mode":"scripted_actions_live_generator","spec":serde_json::from_slice::<Value>(&spec_bytes)?,"source_sha256":env!("FRONTIER_SOURCE_SHA256")}))?;
    println!("Proof run {id}; output {output}");
    let mut checks = vec![];
    let mut captures = BTreeMap::new();
    let result:Result<()>=async {
        for probe in spec.steps {
            let started=std::time::Instant::now();
            {
                let mut store=app.store.lock().unwrap();
                let mut current=store.load(&id)?;
                current.actions+=1;
                store.commit(&mut current,"attempt","Researcher probe",json!({"action":probe.action,"label":probe.label,"actor_inference":false}))?;
            }
            if let Err(error)=construct(app,&id,&probe.action).await {
                {
                    let mut store=app.store.lock().unwrap();
                    let mut current=store.load(&id)?;
                    store.commit(&mut current,"construction_rejected","Probe construction failed",json!({"error":error.to_string()}))?;
                }
                execute_action(app,&id,probe.action,Some(error.to_string())).await?;
                return Err(error);
            }
            execute_action(app,&id,probe.action,None).await?;
            let mut store=app.store.lock().unwrap();
            let mut current=store.load(&id)?;
            let result=&current.world.transcript.last().unwrap()["result"];
            ensure!(result["status"]==200,"Probe {} returned a tool failure: {}",probe.label,result);
            let body:Value=serde_json::from_str(result["body"].as_str().ok_or_else(||anyhow::anyhow!("Expected JSON HTTP response"))?)?;
            let selected=if let Some(pointer)=probe.select {body.pointer(&pointer).ok_or_else(||anyhow::anyhow!("Missing capture pointer {pointer}"))?.clone()} else {body.clone()};
            let matched=contains(&body,&probe.expect) && probe.same_as.as_ref().is_none_or(|label|captures.get(label)==Some(&selected));
            let check=json!({"label":probe.label,"passed":matched,"elapsed_ms":started.elapsed().as_millis(),"expected":probe.expect,"actual":body,"same_as":probe.same_as});
            store.commit(&mut current,"proof_check","Probe assertion",check.clone())?;
            checks.push(check);
            println!("{}: {}",probe.label,if matched {"PASS"} else {"FAIL"});
            ensure!(matched,"Probe {} failed",probe.label);
            captures.insert(probe.label,selected);
        }
        let store=app.store.lock().unwrap();
        let events=store.events(&id)?;
        let latencies:Vec<_>=events.iter().filter(|e|e.kind=="published").filter_map(|e|e.evidence["latency_ms"].as_u64()).collect();
        ensure!(!latencies.is_empty(),"No live construction exercised");
        ensure!(latencies.iter().all(|ms|*ms<=spec.max_generation_ms),"Generation exceeded {} ms acceptance bound: {:?}",spec.max_generation_ms,latencies);
        Ok(())
    }.await;
    let mut store = app.store.lock().unwrap();
    let mut current = store.load(&id)?;
    current.status = if result.is_ok() { "finished" } else { "error" }.into();
    current.world.finished = result.is_ok();
    store.commit(&mut current,"proof_result","JIT proof result",json!({"passed":result.is_ok(),"error":result.as_ref().err().map(ToString::to_string),"max_generation_ms":spec.max_generation_ms}))?;
    let events = store.events(&id)?;
    let report = json!({"mode":"scripted_actions_live_generator","passed":result.is_ok(),"error":result.as_ref().err().map(ToString::to_string),"source_sha256":env!("FRONTIER_SOURCE_SHA256"),"checks":checks,"audit":frontier::audit::report(&current,&events),"run":current,"events":events});
    report_file.write_all(serde_json::to_string_pretty(&report)?.as_bytes())?;
    report_file.sync_all()?;
    result
}
