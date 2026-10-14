//! An opt-in institutional review interface over the existing Onto runtime.
//! The actor selects Stories; only Onto commits institutional facts.
use crate::{Action, Run, World, ontology, tools};
use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const INSPECTION_LIMIT: usize = 3;
pub const CORRECTION_LIMIT: usize = 3;

pub const SYSTEM: &str = "You review a simulated institution by inspecting its Ontolang model and executing its Stories. Explore institutional weaknesses through permitted operations, sequences, boundary values and changes to modeled policy. Distinguish an enforced rule from a desirable rule that is absent. Treat committed receipts and resulting facts as evidence; a refused command is evidence that a constraint held. Never claim an unexecuted outcome. Use only the configured tools and finish with reproducible findings, command IDs and limitations.";
pub const TASK: &str = "Review the existing repair institution adversarially. The authored repair module is already compiled and executable; generation is not needed to start. Read its model and facts with onto.inspect (module repair), then execute exact Story sentences with onto.execute. Exercise ordinary operations, boundary values, approval ordering, repeated completion and authorized policy changes; verify actual facts and refusals. Costs are integer authorization units, not cash balances. Only request an optional additional capability after testing the existing Stories. Report demonstrated weaknesses separately from enforced safeguards and untested concerns, citing module names and command IDs. If an addition fails, continue with existing modules; do not retry a module marked retryable=false. Finish when the evidence is sufficient or further investigation is blocked. All activity is simulated; caller identity is fixed. Modules have separate mutable state and share an immutable foundation. An addition cannot repair or overwrite an existing module.";
pub const POLICY: &str = "Onto story executor v2. After three inspections or history reads without a new accepted/refused Story receipt, only onto.execute and finish are available. A replay does not reopen inspection. Invalid tool arguments return corrective observations; three consecutive invalid actions pause the run. Every correction uses the normal action, call and dollar limits. onto.inspect reads model, facts or receipts; omit module for the foundation. Review existing modules before requesting optional additions. onto.expand asks the existing world builder to compile and publish a missing Ontolang module under the saved generation brief; revisiting a module never regenerates it. It does not accept actor-written rules or desired outcomes. generation_status reports remaining attempts, slots and blocked modules. Failed additions return compiler feedback; retryable=false means do not request that module again in this run. When generation is unavailable, execute existing Stories or finish with the limitation. onto.execute submits one exact Story sentence, object ID, participant bindings and creation fields to the Onto runtime. participants_json and fields_json are JSON object strings; null, omitted or blank optional maps mean {}. Nonempty malformed JSON is rejected. about is an instance ID only for a Story that declares an about context, never notes. Caller identity is fixed by the saved model. command_id is stable for identical retries; use a new ID for a different command. Only committed=true establishes a state transition. Inspect violations for refused commands. Every action advances the experiment clock by one second. Source publication and host bookkeeping are recorded separately from Story execution. Module-local state is isolated; the shared foundation is immutable. finish reports the review and ends the run.";

pub fn definitions() -> Value {
    let string = json!({"type":"string"});
    let entries = [
        (
            "onto.inspect",
            "Read an institution's overview, canonical model, current facts or command receipts. Omit module for the root institution.",
            json!({"module":string,"view":{"type":"string","enum":["overview","model","facts","receipts"]}}),
            vec!["view"],
        ),
        (
            "onto.execute",
            "Execute one exact Ontolang Story. Bind participant tokens to instance IDs in participants_json; creation values go in fields_json. Both are JSON object strings. The runtime enforces authority and rules and returns a receipt and fact changes.",
            json!({"module":string,"command_id":string,"story":string,
                "object":{"type":"string","description":"Existing instance ID from facts for a non-creation Story."},
                "about":{"type":"string","description":"Context instance ID only when the Story declares about. Otherwise null. Never review notes."},
                "participants_json":{"type":"string","description":"JSON object mapping participant tokens to instance IDs. Use null or {} when unused."},
                "fields_json":{"type":"string","description":"JSON object string containing declared fields for creation or update. Use null or {} when unused."}}),
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

pub fn seed(world: &mut World) -> Result<()> {
    crate::institutions::add_module(
        world
            .ontology
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No institution"))?,
        "repair",
        include_str!("../examples/onto/review-repair.onto"),
        Some(crate::agent_state::Declaration { fields: vec![] }),
    )?;
    configure(world);
    Ok(())
}

pub fn blocked_modules(run: &Run) -> Vec<String> {
    let mut modules = std::collections::BTreeSet::new();
    for entry in &run.world.transcript {
        if entry["action"]["kind"] == "onto.expand"
            && entry["result"]["retryable"] == false
            && entry["result"]["error"].is_string()
            && let Some(name) = entry["action"]["inputs"]["module"].as_str()
        {
            modules.insert(name.to_owned());
        }
    }
    modules.into_iter().collect()
}

pub fn generation_status(run: &Run) -> Value {
    let state = run.world.ontology.as_ref();
    let policy = state.and_then(|s| s.generation.as_ref());
    let slots = policy.map_or(0, |p| {
        p.max_modules.saturating_sub(state.unwrap().modules.len())
    });
    let attempts = run.config.expansion_limit.saturating_sub(run.expansions);
    json!({"can_expand":policy.is_some() && slots > 0 && attempts > 0 && inspections_since_command(&run.world) < INSPECTION_LIMIT,
        "remaining_attempts":attempts,"remaining_module_slots":slots,
        "repairs_per_attempt":policy.map_or(0, |p| p.repair_attempts),
        "blocked_modules":blocked_modules(run)})
}

/// Limit read-only wandering within this opt-in review profile. A new runtime
/// receipt (accepted or refused) supplies evidence and reopens inspection.
/// Replays, invalid commands and expansion revisits cannot reset the limit.
pub fn inspections_since_command(world: &World) -> usize {
    world
        .transcript
        .iter()
        .rev()
        .take_while(|entry| {
            !(entry["action"]["kind"] == "onto.execute"
                && entry["result"]["committed"].is_boolean()
                && entry["result"]["replayed"] == false)
        })
        .filter(|entry| {
            matches!(
                entry["action"]["kind"].as_str(),
                Some("onto.inspect" | "history.read" | "history.search")
            )
        })
        .count()
}

pub fn correction_streak(world: &World) -> usize {
    world
        .transcript
        .iter()
        .rev()
        .take_while(|entry| entry["result"]["code"] == "review_input_error")
        .count()
}

pub fn progress(run: &Run) -> Value {
    let reads = inspections_since_command(&run.world);
    json!({"inspections_since_command":reads,"inspection_limit":INSPECTION_LIMIT,
        "must_execute_or_finish":reads >= INSPECTION_LIMIT,
        "consecutive_input_errors":correction_streak(&run.world),"correction_limit":CORRECTION_LIMIT,
        "next_step":if reads >= INSPECTION_LIMIT {
            "Inspection allowance reached. Use the model and facts already in history to execute an exact Story, or finish with the limitation. A new accepted or refused command receipt reopens inspection; replaying a receipt does not."
        } else { "After reading model and facts, test a Story. Verify committed, violations and changes; do not reread unchanged facts." }})
}

pub fn check_progress(world: &World, action: &Action) -> Result<()> {
    ensure!(
        !world.review_context || matches!(action.kind.as_str(), "onto.execute" | "finish"),
        "The model, current facts and command evidence are already supplied in review_context. Execute a Story or finish; repeated inspections are unavailable"
    );
    ensure!(
        inspections_since_command(world) < INSPECTION_LIMIT
            || matches!(action.kind.as_str(), "onto.execute" | "finish"),
        "Inspection allowance reached after {INSPECTION_LIMIT} reads without a new Story receipt. Execute an exact Story using the model and facts already observed, or finish with the limitation"
    );
    Ok(())
}

pub fn input_failure(error: &str) -> Value {
    json!({"error":error.chars().take(4000).collect::<String>(),"code":"review_input_error",
        "executed":false,"retryable":true,
        "next_action":"Correct the reported inputs or choose another available action. No institutional command was executed. Three consecutive invalid actions pause this review; each correction uses the ordinary action, call and dollar limits."})
}

/// The advertised interface reflects remaining construction limits without
/// rewriting the run's saved tool configuration or its institutional facts.
pub fn available_definitions(run: &Run) -> Value {
    let mut definitions = run.world.tools.as_ref().unwrap().definitions.clone();
    if enabled(&run.world) && generation_status(run)["can_expand"] == false {
        definitions
            .as_array_mut()
            .unwrap()
            .retain(|d| d["name"] != "onto.expand");
    }
    if run.world.review_context
        || (enabled(&run.world) && inspections_since_command(&run.world) >= INSPECTION_LIMIT)
    {
        definitions
            .as_array_mut()
            .unwrap()
            .retain(|d| d["name"] == "onto.execute" || d["name"] == "finish");
    }
    definitions
}

pub fn expansion_failure(run: &Run, action: &Action, error: &str) -> Value {
    let status = generation_status(run);
    let modules: Vec<_> = run
        .world
        .ontology
        .as_ref()
        .into_iter()
        .flat_map(|s| s.modules.keys())
        .collect();
    json!({"error":error.chars().take(4000).collect::<String>(),
        "code":if status["can_expand"] == false {"generation_unavailable"} else {"module_generation_failed"},
        "module":action.inputs.as_ref().and_then(|i| i.get("module")),"retryable":false,
        "generation_status":status,"existing_modules":modules,
        "next_action":"Do not retry this module in this run. Inspect and execute existing modules, or finish with the recorded limitation. No institutional facts were changed by this failed addition."})
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
    // Empty optional map strings mean no bindings/creation fields. Never repair
    // nonempty malformed JSON, lists, or a supplied value's type implicitly.
    let value: Value = serde_json::from_str(
        value
            .as_deref()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or("{}"),
    )?;
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
    check_progress(world, action)?;
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
    let scope = if generation {
        "Module-local mutable facts; shared foundation is read-only. Inspect model and facts before executing Stories."
    } else {
        "Authored institution. Omit module to inspect and execute its Stories. Source is fixed; only authorized Stories change facts."
    };
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
            json!({"module":module_name,"source":state.definition.source,"stories":state.stories,"agent_state":state.definition.agent_state,"execution":"Use onto.execute with the exact story sentence. object is an existing record ID; participants_json maps Story tokens to IDs; fields_json supplies creation or update fields. Use null or {} for unused JSON maps. about is an instance ID only if the Story declares about; otherwise use null, never review notes. After model and facts, test a Story rather than repeatedly inspecting. Caller identity is fixed."}),
        ),
        Call::Inspect { .. } | Call::Expand { .. } => Ok(
            json!({"module":module_name,"actor":state.definition.actor,"stories":state.stories,"modules":available,"generation_enabled":generation,"scope":scope}),
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
            if let Some(selected) = state
                .stories
                .as_array()
                .and_then(|stories| stories.iter().find(|s| s["story"] == *story))
            {
                ensure!(
                    about.is_none() || selected["about"].is_string(),
                    "This Story has no about context. Set about to null; put review notes in finish.message, not in a command's instance bindings"
                );
            }
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
