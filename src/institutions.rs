//! Open-ended, lazy executable modules. The host owns a small read-only
//! foundation; generated Onto owns each module's local behavior and state.
use crate::{
    Run, World,
    construction::{Directive, Request, Resource},
    ontology::{self, State},
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub brief: String,
    pub max_modules: usize,
}

pub fn enable(state: &mut State, policy: Policy) -> Result<()> {
    ensure!(
        !policy.brief.trim().is_empty() && policy.brief.len() <= 8000,
        "Generation brief must be 1-8000 bytes"
    );
    ensure!(
        (1..=128).contains(&policy.max_modules),
        "Module bound must be 1-128"
    );
    ensure!(
        state.stories.as_array().is_some_and(Vec::is_empty) && state.receipts.is_empty(),
        "Generative foundation must be immutable and have no Stories"
    );
    ensure!(
        serde_json::to_vec(&state.definition.source)?.len() <= 16000,
        "Foundation source exceeds 16 KiB"
    );
    state.protected = state.snapshot["instances"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Invalid foundation snapshot"))?
        .clone();
    state.protected_types = state
        .protected
        .iter()
        .filter_map(|v| v["type"].as_str().map(str::to_owned))
        .collect();
    state.protected_types.sort();
    state.protected_types.dedup();
    state.generation = Some(policy);
    Ok(())
}

pub fn module_path(path: &str) -> Option<(&str, &str)> {
    let rest = path.strip_prefix("/modules/")?;
    let (key, route) = rest.split_once('/').map_or((rest, "/"), |(k, r)| {
        (
            k,
            match r {
                "model" => "/model",
                "facts" => "/facts",
                "commands" => "/commands",
                _ => "invalid",
            },
        )
    });
    (!key.is_empty()
        && key.len() <= 48
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && route != "invalid")
        .then_some((key, route))
}

pub fn exists(state: &State, path: &str) -> bool {
    if ["/", "/model", "/facts", "/commands"].contains(&path) {
        return true;
    }
    module_path(path).is_some_and(|(key, _)| state.modules.contains_key(key))
}

pub fn directive(run: &Run, target: &str) -> Option<Directive> {
    let state = run.world.ontology.as_ref()?;
    state.generation.as_ref()?;
    let path = ontology::route(target)?;
    let (key, route) = module_path(&path)?;
    if route != "/" || state.modules.contains_key(key) {
        return None;
    }
    Some(crate::experiments::directive(
        target,
        "Construct this previously unspecified institutional capability as executable Onto. Return initial_state {source: <one Onto source string>}. The host supplies the committed foundation unchanged. Add new objects, rules, formulas, seeds and single-actor Stories. Do not redeclare or seed foundation types. Use the existing actor. Model actual operations and authorized restructuring through Stories, not prose. All new state is local to this module; do not claim to have changed another module or the shared foundation. No external effects, System, every, imports or JavaScript. Preserve foundation commitments. The capability name is a request for subject matter, not authority to bypass rules.",
        json!({"functionality":"onto.module.v1","dependencies":[ontology::ROOT],"module":key,"source_max_bytes":32768,"authoring":AUTHORING}),
    ))
}

pub fn context(run: &Run) -> Result<Value> {
    let state = run
        .world
        .ontology
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No institution"))?;
    ensure!(
        state.modules.len()
            < state
                .generation
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Generation disabled"))?
                .max_modules,
        "Module budget exhausted"
    );
    Ok(
        json!({"foundation_source":state.definition.source,"foundation_facts":state.snapshot,"actor":state.definition.actor,"brief":state.generation.as_ref().ok_or_else(|| anyhow::anyhow!("Generation disabled"))?.brief,"isolation":"Only this module's local facts are writable. Shared foundation declarations and facts are immutable. Other modules are not implicitly visible or writable."}),
    )
}

pub fn publish(world: &mut World, request: &Request, resource: &Resource) -> Result<()> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Payload {
        source: String,
    }
    let payload: Payload = serde_json::from_value(resource.initial_state.clone())?;
    ensure!(
        !payload.source.is_empty() && payload.source.len() <= 32768,
        "Module source exceeds bounds"
    );
    let state = world
        .ontology
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("No institution"))?;
    let policy = state
        .generation
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Generation disabled"))?;
    ensure!(
        state.modules.len() < policy.max_modules,
        "Module budget exhausted"
    );
    let path = ontology::route(&resource.target)
        .ok_or_else(|| anyhow::anyhow!("Invalid institution target"))?;
    let (key, route) = module_path(&path).ok_or_else(|| anyhow::anyhow!("Invalid module path"))?;
    ensure!(
        route == "/"
            && request.directive.requirements["module"] == key
            && !state.modules.contains_key(key),
        "Invalid or committed module"
    );
    let mut source = state.definition.source.clone();
    ensure!(
        !source.contains_key("generated.onto"),
        "Foundation reserves generated.onto"
    );
    source.insert("generated.onto".into(), payload.source);
    let response = ontology::bridge(
        json!({"operation":"initialize","source":source,"foundation":state.definition.source,"actor":state.definition.actor,"epoch":state.definition.epoch,"elapsed_seconds":0}),
        &state.runtime_sha256,
    )?;
    ensure!(
        response["stories"]
            .as_array()
            .is_some_and(|s| !s.is_empty()),
        "Generated module must expose executable Stories"
    );
    ensure!(
        serde_json::to_vec(&response["snapshot"])?.len() <= 262144,
        "Module state exceeds 256 KiB"
    );
    let mut definition = state.definition.clone();
    definition.source = serde_json::from_value(response["source"].clone())?;
    state.modules.insert(
        key.into(),
        State {
            definition,
            runtime_sha256: state.runtime_sha256.clone(),
            snapshot: response["snapshot"].clone(),
            stories: response["stories"].clone(),
            receipts: BTreeMap::new(),
            generation: None,
            modules: BTreeMap::new(),
            protected: serde_json::from_value(response["protected"].clone())?,
            protected_types: serde_json::from_value(response["protected_types"].clone())?,
        },
    );
    Ok(())
}

// Concise syntax guidance, not a prewritten capability implementation.
pub const AUTHORING: &str = r#"Onto syntax: object CamelCase then indented fields, e.g. name: string @required, amount: int @required, policy: Policy @required. Type constraints use @rule Type.amount is at least 0. A derived field uses field: int = expression. Foundation actors already exist: do not redeclare them. story Title contains indented Actor verb Object, then further indented requires Object.field is ... and sets Object.field to ... . Use creates for creation; a gate on the new object's fields can make a beat refer to an existing object instead, so constrain creation with type @rule or an existing participant. A creation participant is written Actor creates Item for Policy, with sets Item.policy to Policy. Policies can be objects modified by Stories using sets; guards can compare Item.amount is at most Item.policy.limit. Mutation must write an explicit sets clause. A seed block is seed Type followed by one row per instance, e.g. name: "main", limit: 10. Seed references use the named record's name. Runtime commands use Type:name IDs and integer stored units. A Story-written status enum needs complete lifecycle and @final ending; use numeric or boolean fields if no lifecycle is needed. Booleans are true/false. Numbers use int, not integer or number. Cross-type links should have mirrors. Every named participant must be used in a clause. Every clause type must be bound by the sentence, a stored path, or finds. No System or every Stories. No external effects. No imports. Do not output markdown fences. Follow the JSON proposal envelope exactly."#;
