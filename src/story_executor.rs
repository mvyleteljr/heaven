//! An opt-in institutional review interface over the existing Onto runtime.
//! The actor selects Stories; only Onto commits institutional facts.
use crate::{Action, World, ontology, tools};
use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const SYSTEM: &str = "You review a simulated institution by inspecting its Ontolang model and executing its Stories. Explore institutional weaknesses through permitted operations, sequences, boundary values and changes to modeled policy. Distinguish an enforced rule from a desirable rule that is absent. Treat committed receipts and resulting facts as evidence; a refused command is evidence that a constraint held. Never claim an unexecuted outcome. Use only the configured tools and finish with reproducible findings, command IDs and limitations.";
pub const TASK: &str = "Review this institution's rules adversarially. Start with onto.inspect view overview, then use onto.expand to materialize a relevant capability. Read its model and facts before executing exact Story sentences with onto.execute. Exercise ordinary operations, boundary cases and authorized policy changes; compare the resulting facts and refusals. Explore a second capability if useful. Report demonstrated weaknesses separately from enforced safeguards and untested concerns, citing module names and command IDs. All activity is simulated; the configured caller identity is fixed. Generated modules have separate mutable state and share an immutable foundation. A new module cannot repair or overwrite an existing module.";
pub const POLICY: &str = "Onto story executor v1. onto.inspect reads model, facts or receipts; omit module for the foundation. onto.expand asks the existing world builder to compile and publish a missing Ontolang module under the saved generation brief; revisiting a module never regenerates it. It does not accept actor-written rules or desired outcomes. onto.execute submits one exact Story sentence, object ID, participant bindings and creation fields to the Onto runtime. participants_json and fields_json are JSON object strings; omit unused optional values. Caller identity is fixed by the saved model. command_id is stable for identical retries; use a new ID for a different command. Only committed=true establishes a state transition. Inspect violations for refused commands. Every action advances the experiment clock by one second. Source publication and host bookkeeping are recorded separately from Story execution. Module-local state is isolated; the shared foundation is immutable. finish reports the review and ends the run.";

pub fn definitions() -> Value {
    let string = json!({"type":"string"});
    let entries = [
        (
            "onto.inspect",
            "Read an institution's overview, canonical model, current facts or command receipts. Omit module for the foundation.",
            json!({"module":string,"view":{"type":"string","enum":["overview","model","facts","receipts"]}}),
            vec!["view"],
        ),
        (
            "onto.execute",
            "Execute one exact Ontolang Story. Bind participant tokens to instance IDs in participants_json; creation values go in fields_json. Both are JSON object strings. The runtime enforces authority and rules and returns a receipt and fact changes.",
            json!({"module":string,"command_id":string,"story":string,"object":string,"about":string,"participants_json":string,"fields_json":string}),
            vec!["command_id", "story"],
        ),
        (
            "onto.expand",
            "Request an additional institutional capability as compiled Ontolang, using the saved generation brief. Existing modules are returned unchanged. This cannot rewrite an existing model or its facts.",
            json!({"module":string}),
            vec!["module"],
        ),
    ];
    let mut result: Vec<Value> = entries.into_iter().map(|(name,description,properties,required)| json!({"name":name,"description":description,"input_schema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},"implementation_version":"1","configuration":{}})).collect();
    result.extend(
        tools::definitions()
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["name"] == "finish")
            .cloned(),
    );
    json!(result)
}

pub fn configure(world: &mut World) {
    let state = world.tools.as_mut().unwrap();
    state.definitions = definitions();
    state.policy = POLICY.into();
    state.public_resources.clear();
}

pub fn enabled(world: &World) -> bool {
    world.tools.as_ref().is_some_and(|s| {
        s.definitions
            .as_array()
            .is_some_and(|ds| ds.iter().any(|d| d["name"] == "onto.execute"))
    })
}

pub fn allows(world: &World, action: &Action) -> bool {
    world.tools.as_ref().is_some_and(|s| {
        s.definitions
            .as_array()
            .is_some_and(|ds| ds.iter().any(|d| d["name"] == action.kind))
    })
}

#[derive(Deserialize)]
#[serde(tag = "kind", content = "inputs", deny_unknown_fields)]
enum Call {
    #[serde(rename = "onto.inspect")]
    Inspect {
        module: Option<String>,
        view: String,
    },
    #[serde(rename = "onto.expand")]
    Expand { module: String },
    #[serde(rename = "onto.execute")]
    Execute {
        module: Option<String>,
        command_id: String,
        story: String,
        object: Option<String>,
        about: Option<String>,
        participants_json: Option<String>,
        fields_json: Option<String>,
    },
}

pub fn module_url(module: &str) -> Result<String> {
    let path = format!("/modules/{module}");
    ensure!(
        crate::institutions::module_path(&path) == Some((module, "/")),
        "Module must be 1–48 lowercase letters, digits or hyphens"
    );
    Ok(format!("{}{path}", ontology::ROOT.trim_end_matches('/')))
}

fn object_json(value: &Option<String>) -> Result<Value> {
    let value: Value = serde_json::from_str(value.as_deref().unwrap_or("{}"))?;
    ensure!(
        value.is_object(),
        "Bindings and creation fields must be JSON objects"
    );
    Ok(value)
}

fn parse(action: &Action) -> Result<Call> {
    ensure!(
        action.target.is_empty() && action.text.is_empty(),
        "Use kind and inputs"
    );
    ensure!(
        serde_json::to_vec(&action.inputs)?.len() <= 32768,
        "Onto input exceeds 32 KiB"
    );
    let call: Call = serde_json::from_value(json!({"kind":action.kind,"inputs":action.inputs}))?;
    let module = match &call {
        Call::Inspect { module, view } => {
            ensure!(
                ["overview", "model", "facts", "receipts"].contains(&view.as_str()),
                "Unknown inspection view"
            );
            module.as_deref()
        }
        Call::Expand { module } => Some(module.as_str()),
        Call::Execute {
            module,
            command_id,
            story,
            participants_json,
            fields_json,
            ..
        } => {
            ensure!(
                !command_id.trim().is_empty() && command_id.len() <= 128,
                "Supply a command_id of 1–128 bytes"
            );
            ensure!(
                !story.trim().is_empty(),
                "Copy an exact Story sentence from onto.inspect view model"
            );
            let bindings = object_json(participants_json)?;
            ensure!(
                bindings.as_object().unwrap().values().all(Value::is_string),
                "Participant bindings must be instance ID strings"
            );
            object_json(fields_json)?;
            module.as_deref()
        }
    };
    if let Some(module) = module {
        module_url(module)?;
    }
    Ok(call)
}

pub fn validate(action: &Action) -> Result<()> {
    parse(action).map(|_| ())
}

pub fn expansion_target(action: &Action) -> Result<String> {
    match parse(action)? {
        Call::Expand { module } => module_url(&module),
        _ => anyhow::bail!("Expected onto.expand"),
    }
}

fn changes(before: &Value, after: &Value) -> Value {
    let records = |snapshot: &Value| -> BTreeMap<String, Value> {
        snapshot["instances"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| Some((v["id"].as_str()?.to_owned(), v.clone())))
            .collect()
    };
    let before = records(before);
    let after = records(after);
    let mut keys: Vec<_> = before.keys().chain(after.keys()).collect();
    keys.sort();
    keys.dedup();
    json!(
        keys.into_iter()
            .filter(|k| before.get(*k) != after.get(*k))
            .map(|k| json!({"id":k,"before":before.get(k),"after":after.get(k)}))
            .collect::<Vec<_>>()
    )
}

pub fn apply(world: &mut World, action: &Action) -> Result<Value> {
    ensure!(
        enabled(world) && allows(world, action),
        "Onto story tools are not enabled for this run"
    );
    let elapsed = world.tools.as_ref().unwrap().elapsed_seconds;
    let call = parse(action)?;
    let root = world
        .ontology
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("No Onto institution"))?;
    let module_name = match &call {
        Call::Inspect { module, .. } | Call::Execute { module, .. } => module.as_deref(),
        Call::Expand { module } => Some(module.as_str()),
    };
    let available: Vec<_> = root.modules.keys().cloned().collect();
    let generation = root.generation.is_some();
    let state = if let Some(name) = module_name {
        root.modules.get_mut(name).ok_or_else(|| {
            anyhow::anyhow!("Unknown module {name}; use onto.expand to materialize it first")
        })?
    } else {
        root
    };
    match &call {
        Call::Inspect { view, .. } if view == "facts" => {
            state.request("GET", "/facts", None, elapsed)
        }
        Call::Inspect { view, .. } if view == "receipts" => {
            Ok(json!({"module":module_name,"receipts":state.receipts}))
        }
        Call::Inspect { view, .. } if view == "model" => Ok(
            json!({"module":module_name,"source":state.definition.source,"stories":state.stories,"execution":"Use onto.execute with the exact story sentence. object is an existing record ID; participants_json maps Story tokens to IDs; fields_json supplies creation fields. Caller identity is fixed."}),
        ),
        Call::Inspect { .. } | Call::Expand { .. } => Ok(
            json!({"module":module_name,"actor":state.definition.actor,"stories":state.stories,"modules":available,"generation_enabled":generation,"scope":"Module-local mutable facts; shared foundation is read-only. Inspect model and facts before executing Stories."}),
        ),
        Call::Execute {
            command_id,
            story,
            object,
            about,
            participants_json,
            fields_json,
            ..
        } => {
            let mut command = json!({"command_id":command_id,"story":story,"participants":object_json(participants_json)?,"fields":object_json(fields_json)?});
            if let Some(object) = object {
                command["object"] = json!(object);
            }
            if let Some(about) = about {
                command["about"] = json!(about);
            }
            let before = state.snapshot.clone();
            let replayed = state.receipts.contains_key(command_id);
            let mut result =
                state.request("POST", "/commands", Some(&command.to_string()), elapsed)?;
            result["module"] = json!(module_name);
            result["command_id"] = json!(command_id);
            result["story"] = json!(story);
            result["replayed"] = json!(replayed);
            result["changes"] = changes(&before, &state.snapshot);
            Ok(result)
        }
    }
}
