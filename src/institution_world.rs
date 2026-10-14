//! Connected institution: one authoritative snapshot, scoped writes, bounded
//! host events and versioned amendments. Legacy isolated runs remain unchanged.
use crate::{
    Action, Run, World,
    construction::{Directive, Interface, Request, Resolution, Resource},
    ontology::{self, State},
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const FUNCTIONALITY: &str = "onto.connected.v1";
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Seed-owned authority; generated source cannot change this list.
    pub governors: Vec<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Connected {
    pub modules: BTreeMap<String, Module>,
    pub owners: BTreeMap<String, String>,
    pub story_owners: BTreeMap<String, String>,
    pub fields: Value,
    pub receipts: BTreeMap<String, Value>,
    pub revisions: Vec<Value>,
    pub provenance: Vec<Value>,
    pub elapsed: u64,
    pub standing: Vec<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Module {
    pub source: String,
    pub agent_state: crate::agent_state::Declaration,
    pub grants: BTreeMap<String, Vec<String>>,
    pub events: Vec<Event>,
    pub revision: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub id: String,
    pub actor: String,
    pub at: u64,
    pub every_seconds: u64,
    pub remaining: u32,
    pub command: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    source: String,
    agent_state: crate::agent_state::Declaration,
    grants: BTreeMap<String, Vec<String>>,
    events: Vec<Event>,
    migration: Vec<Value>,
    error: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Amendment {
    expected_revision: u64,
    mandate: String,
}

pub fn enabled(state: &State) -> bool {
    state.connected.is_some()
}
pub fn enable(state: &mut State) -> Result<()> {
    let policy = state
        .generation
        .as_ref()
        .and_then(|p| p.connected.as_ref())
        .unwrap();
    ensure!(
        state.modules.is_empty(),
        "Cannot silently migrate isolated modules"
    );
    ensure!(
        policy.governors.len() <= 16
            && policy.governors.iter().all(|a| state.snapshot["instances"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["id"] == *a)),
        "Governors must be seeded identities"
    );
    // Fail at setup rather than silently accepting an older pinned runtime.
    let mut c = Connected::default();
    let response = evaluate(
        state,
        &c,
        0,
        Evaluation {
            mode: "inspect",
            module: "",
            actor: &state.definition.actor,
            command: None,
            migration: &[],
        },
    )?;
    c.owners = serde_json::from_value(response["owners"].clone())?;
    c.fields = response["fields"].clone();
    state.connected = Some(c);
    Ok(())
}
struct Evaluation<'a> {
    mode: &'a str,
    module: &'a str,
    actor: &'a str,
    command: Option<Value>,
    migration: &'a [Value],
}
fn evaluate(
    state: &State,
    connected: &Connected,
    elapsed: u64,
    evaluation: Evaluation<'_>,
) -> Result<Value> {
    let Evaluation {
        mode,
        module,
        actor,
        command,
        migration,
    } = evaluation;
    let parts: BTreeMap<_, _> = connected
        .modules
        .iter()
        .map(|(k, m)| (k, &m.source))
        .collect();
    ontology::bridge(
        json!({"operation":"connected","mode":mode,"foundation":state.definition.source,"parts":parts,"snapshot":state.snapshot,"epoch":state.definition.epoch,"elapsed_seconds":elapsed,"module":module,"actor":actor,"command":command,"host_event":actor != state.definition.actor,"migration":migration,"previous_parts":state.connected.as_ref().map(|c|c.modules.iter().map(|(k,m)|(k,&m.source)).collect::<BTreeMap<_,_>>())}),
        &state.runtime_sha256,
    )
}
fn revision_path(path: &str) -> Option<(&str, &str)> {
    let rest = path.strip_prefix("/modules/")?;
    let (name, id) = rest.split_once("/revisions/")?;
    if crate::institutions::module_path(&format!("/modules/{name}")).is_none()
        || id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
    {
        return None;
    }
    Some((name, id))
}
pub fn resolve(run: &Run, action: &Action) -> Result<Option<Resolution>> {
    let Some(state) = run.world.ontology.as_ref().filter(|s| enabled(s)) else {
        return Ok(None);
    };
    if action.kind == "onto.expand"
        || (action.kind == "onto.inspect"
            && action
                .inputs
                .as_ref()
                .and_then(|i| i["module"].as_str())
                .is_some())
    {
        crate::story_executor::validate(action)?;
        let module = action.inputs.as_ref().unwrap()["module"].as_str().unwrap();
        if state
            .connected
            .as_ref()
            .unwrap()
            .modules
            .contains_key(module)
        {
            return Ok(Some(Resolution::Existing));
        }
        return resolve(
            run,
            &crate::tools::call(
                "http.request",
                json!({"method":"GET","url":format!("https://institutions.local/modules/{module}")}),
            ),
        );
    }
    if action.kind != "http.request" {
        return Ok(None);
    }
    crate::tools::validate_action(action)?;
    let i = action.inputs.as_ref().unwrap();
    let Some(path) = i["url"].as_str().and_then(ontology::route) else {
        return Ok(None);
    };
    let mut requirements = json!({"functionality":FUNCTIONALITY,"dependencies":[ontology::ROOT],"repair_attempts":state.generation.as_ref().unwrap().repair_attempts,"authoring":format!("{}\n{}\n{}", crate::institutions::AUTHORING.replace("No System or every Stories.", "System Stories execute only through host schedules; no native every Stories.").replace("Return only the source/error response schema.", "Return the connected payload described by the directive."), include_str!("agent_state_guidance.md"), include_str!("onto_grammar.md"))});
    let (module, operation) = if let Some((module, id)) = revision_path(&path) {
        ensure!(i["method"] == "POST", "Amendments require POST");
        if state
            .connected
            .as_ref()
            .unwrap()
            .revisions
            .iter()
            .any(|r| r["target"] == i["url"])
        {
            return Ok(Some(Resolution::Existing));
        }
        let amendment: Amendment = serde_json::from_str(i["body"].as_str().unwrap_or("{}"))?;
        let m = state
            .connected
            .as_ref()
            .unwrap()
            .modules
            .get(module)
            .ok_or_else(|| anyhow::anyhow!("Unknown module"))?;
        ensure!(
            m.revision == amendment.expected_revision,
            "Stale amendment revision"
        );
        ensure!(
            !amendment.mandate.trim().is_empty() && amendment.mandate.len() <= 4000,
            "Amendment mandate must be 1-4000 bytes"
        );
        ensure!(
            state
                .generation
                .as_ref()
                .unwrap()
                .connected
                .as_ref()
                .unwrap()
                .governors
                .contains(&state.definition.actor),
            "Caller has no amendment authority"
        );
        requirements["expected_revision"] = json!(m.revision);
        requirements["mandate"] = json!(amendment.mandate);
        requirements["amendment_id"] = json!(id);
        requirements["submitted"] = json!(i["body"]);
        (module.to_owned(), "revise")
    } else if let Some((name, route)) = crate::institutions::module_path(&path) {
        if i["method"] != "GET"
            || route == "/commands"
            || state.connected.as_ref().unwrap().modules.contains_key(name)
        {
            return Ok(None);
        }
        (name.to_owned(), "create")
    } else {
        return Ok(None);
    };
    requirements["module"] = json!(module);
    let target = if operation == "create" {
        crate::story_executor::module_url(&module)?
    } else {
        i["url"].as_str().unwrap().into()
    };
    Ok(Some(Resolution::Missing {
        directive: Directive {
            operation: operation.into(),
            target,
            instruction: GUIDANCE.into(),
            requirements,
        },
        interface: Interface {
            tool: action.kind.clone(),
            method: i["method"].as_str().unwrap().into(),
            url: i["url"].as_str().unwrap().into(),
        },
    }))
}
pub fn context(state: &State) -> Value {
    let c = state.connected.as_ref().unwrap();
    // Supply the live program, not a growing duplicate of every historical delta.
    json!({"foundation_source":state.definition.source,"snapshot":state.snapshot,"modules":c.modules,"owners":c.owners,"stories":state.stories,"fields":c.fields,"elapsed_seconds":c.elapsed,"actor":state.definition.actor,"brief":state.generation.as_ref().unwrap().brief,"scope":GUIDANCE})
}
pub const GUIDANCE: &str = "Build a connected fictional institution. Return source, agent_state, grants, events, migration and error. Reuse the complete existing vocabulary and live facts supplied in context. Source contains ONLY this module's declarations and Stories. All modules read ONE shared snapshot. A module owns its declared types and Stories; it can write its own records, or another type only when its owner grants this module permission. Grants map owned type names to allowed module names. Never redeclare foundation or sibling types/Stories. Use their types and fields directly in references, guards and formulas; no imports needed. Foundation is immutable. Use bounded events for institutional behavior: id, actor (a seeded local counterparty or local System actor type), at (absolute world seconds strictly after current elapsed), every_seconds (0 for one-shot), remaining (1-100), command (story plus optional object/participants/fields). Events execute existing local Stories under host authority, never the player identity. Native every and external effects are unsupported. Empty events/grants are valid. Revisions replace only this module source, preserve existing facts and ownership, and implement the authorized mandate; migration contains explicit {id,fields} patches to owned records where needed. No deletion or implicit seed reset. Adding required fields needs explicit migration for existing records. Keep existing declarations unless the amendment requires a compatible change. Agent state binds real local owned records; do not invent beliefs. Do not prescribe success. Return error only for actual contradictions. The general Onto syntax guidance applies except the old module-isolation and System prohibitions: this connected profile supports shared reads and host-only System events.";

pub fn response_format() -> Value {
    // Detailed payload validation is host-owned. JSON object transport avoids
    // provider-specific grammar compilation for arbitrary Onto field dictionaries.
    json!({"type":"json_object"})
}
pub fn proposal(request: &Request, response: &str) -> Result<crate::construction::Proposal> {
    let payload: Payload = serde_json::from_str(response)?;
    ensure!(
        payload.error.is_none(),
        "Generation declined: {:?}",
        payload.error
    );
    let value: Value = serde_json::from_str(response)?;
    Ok(crate::construction::Proposal {
        version: 1,
        request_id: request.request_id.clone(),
        basis: request.basis.clone(),
        result: crate::construction::Outcome::Proposed {
            resources: vec![Resource {
                target: request.directive.target.clone(),
                operation: request.directive.operation.clone(),
                initial_state: value,
                interface: request.interface.clone(),
                functionality: FUNCTIONALITY.into(),
                dependencies: vec![ontology::ROOT.into()],
            }],
        },
    })
}
fn writable(c: &Connected, module: &str, typ: &str) -> bool {
    c.owners.get(typ).is_some_and(|owner| {
        owner == module
            || c.modules
                .get(owner)
                .and_then(|m| m.grants.get(typ))
                .is_some_and(|xs| xs.iter().any(|x| x == module))
    })
}
fn stored(row: &Value, fields: &Value) -> Value {
    let mut row = row.clone();
    if let Some(values) = row["fields"].as_object_mut() {
        values.retain(|k, _| {
            !fields.as_array().is_some_and(|fs| {
                fs.iter()
                    .any(|f| f["name"] == *k && (!f["computed"].is_null() || f["type"] == "link"))
            })
        });
    }
    row
}
fn changes(before: &Value, after: &Value) -> Vec<Value> {
    let old: BTreeMap<_, _> = before["instances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["id"].as_str().unwrap(), r))
        .collect();
    let new: BTreeMap<_, _> = after["instances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["id"].as_str().unwrap(), r))
        .collect();
    let mut ids: Vec<_> = old.keys().chain(new.keys()).copied().collect();
    ids.sort();
    ids.dedup();
    ids.into_iter()
        .filter(|id| old.get(id) != new.get(id))
        .map(|id| json!({"id":id,"before":old.get(id),"after":new.get(id)}))
        .collect()
}
fn project(state: &mut State, c: &Connected, response: &Value) -> Result<()> {
    ensure!(
        serde_json::to_vec(&response["snapshot"])?.len() <= 262144,
        "Shared snapshot exceeds 256 KiB"
    );
    state.check_protected(&response["snapshot"])?;
    state.accept_snapshot(&response["snapshot"])?;
    state.stories = response["stories"].clone();
    for (name, m) in &c.modules {
        let mut definition = state.definition.clone();
        definition.source = BTreeMap::from([(format!("{name}.onto"), m.source.clone())]);
        definition.agent_state = Some(m.agent_state.clone());
        let view = State {
            connected: None,
            definition,
            runtime_sha256: state.runtime_sha256.clone(),
            snapshot: json!({"instances":state.snapshot["instances"].as_array().unwrap().iter().filter(|r|c.owners.get(r["type"].as_str().unwrap()).is_some_and(|o|o==name)).collect::<Vec<_>>(),"now":state.snapshot["now"]}),
            agent_state: Some(
                m.agent_state
                    .project(&state.definition.actor, &state.snapshot)?,
            ),
            stories: json!(
                response["stories"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|s| c.story_owners.get(s["story"].as_str().unwrap()) == Some(name))
                    .collect::<Vec<_>>()
            ),
            receipts: BTreeMap::new(),
            generation: None,
            modules: BTreeMap::new(),
            protected: vec![],
            protected_types: vec![],
        };
        state.modules.insert(name.clone(), view);
    }
    Ok(())
}
pub fn publish(world: &mut World, request: &Request, resource: &Resource) -> Result<()> {
    let state = world.ontology.as_mut().unwrap();
    let mut c = state.connected.clone().unwrap();
    let name = request.directive.requirements["module"].as_str().unwrap();
    let p: Payload = serde_json::from_value(resource.initial_state.clone())?;
    ensure!(
        p.error.is_none() && !p.source.trim().is_empty() && p.source.len() <= 32768,
        "Invalid connected source"
    );
    ensure!(
        c.revisions.len() < 128 && c.modules.len() < state.generation.as_ref().unwrap().max_modules
            || c.modules.contains_key(name) && c.revisions.len() < 128,
        "Institution publication limit"
    );
    let revision = if request.directive.operation == "revise" {
        ensure!(
            state
                .generation
                .as_ref()
                .unwrap()
                .connected
                .as_ref()
                .unwrap()
                .governors
                .contains(&state.definition.actor),
            "No amendment authority"
        );
        let old = c
            .modules
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Missing module"))?;
        ensure!(
            json!(old.revision) == request.directive.requirements["expected_revision"],
            "Stale revision"
        );
        old.revision + 1
    } else {
        ensure!(
            !c.modules.contains_key(name) && p.migration.is_empty(),
            "Create cannot migrate existing state"
        );
        1
    };
    let old_owners = c.owners.clone();
    ensure!(
        p.events.len() <= 32 && p.migration.len() <= 128,
        "Publication too large"
    );
    c.modules.insert(
        name.into(),
        Module {
            source: p.source,
            agent_state: p.agent_state,
            grants: p.grants,
            events: p.events,
            revision,
        },
    );
    let now = world.tools.as_ref().map_or(0, |t| t.elapsed_seconds);
    let response = evaluate(
        state,
        &c,
        now,
        Evaluation {
            mode: "compose",
            module: name,
            actor: &state.definition.actor,
            command: None,
            migration: &p.migration,
        },
    )?;
    c.owners = serde_json::from_value(response["owners"].clone())?;
    c.story_owners = serde_json::from_value(response["story_owners"].clone())?;
    c.fields = response["fields"].clone();
    c.standing = serde_json::from_value(response["standing"].clone())?;
    for (typ, owner) in &old_owners {
        ensure!(
            c.owners.get(typ) == Some(owner),
            "Cannot remove or transfer existing type ownership"
        );
    }
    for (typ, grantees) in &c.modules[name].grants {
        ensure!(
            c.owners.get(typ).is_some_and(|o| o == name) && grantees.len() <= 32,
            "Only type owner may grant writes"
        );
    }
    ensure!(
        c.modules
            .values()
            .map(|m| m.events.iter().filter(|e| e.remaining > 0).count())
            .sum::<usize>()
            <= 64,
        "At most 64 active institution events"
    );
    let m = &c.modules[name];
    for field in &m.agent_state.fields {
        ensure!(
            response["snapshot"]["instances"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["id"] == field.source.instance
                    && c.owners
                        .get(r["type"].as_str().unwrap())
                        .is_some_and(|o| o == name)),
            "Agent projection outside module"
        );
    }
    let mut ids = std::collections::BTreeSet::new();
    for event in &m.events {
        ensure!(
            !event.id.is_empty()
                && event.id.len() <= 32
                && ids.insert(&event.id)
                && event.at > now
                && event.remaining > 0
                && event.remaining <= 100
                && (event.remaining == 1 || event.every_seconds > 0),
            "Invalid event schedule"
        );
        ensure!(
            event
                .every_seconds
                .checked_mul(event.remaining as u64)
                .and_then(|n| event.at.checked_add(n))
                .is_some(),
            "Event clock overflow"
        );
        validate_event(state, &c, &response, name, event)?;
    }
    let delta = changes(&state.snapshot, &response["snapshot"]);
    c.revisions.push(json!({"target":resource.target,"module":name,"revision":revision,"at":now,"source_sha256":format!("{:x}",Sha256::digest(&m.source)),"mandate":request.directive.requirements["mandate"],"submitted":request.directive.requirements["submitted"],"migration":p.migration,"changes":delta}));
    c.provenance.push(
        json!({"kind":"publication","module":name,"revision":revision,"at":now,"changes":delta}),
    );
    project(state, &c, &response)?;
    state.connected = Some(c);
    Ok(())
}
fn validate_event(
    state: &State,
    c: &Connected,
    response: &Value,
    module: &str,
    e: &Event,
) -> Result<()> {
    ensure!(
        e.actor != state.definition.actor,
        "Schedules cannot impersonate the player"
    );
    ensure!(
        c.story_owners
            .get(e.command["story"].as_str().unwrap_or(""))
            .is_some_and(|o| o == module),
        "Event Story outside module"
    );
    let actor_type = e.actor.split(':').next().unwrap();
    ensure!(
        c.owners.get(actor_type).is_some_and(|o| o == module),
        "Event actor must belong to this module"
    );
    let system = response["stories"].as_array().unwrap().iter().any(|s| {
        s["story"] == e.command["story"] && s["system"] == true && s["actor"] == actor_type
    });
    ensure!(
        system
            || response["snapshot"]["instances"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["id"] == e.actor),
        "Event actor is not seeded"
    );
    ensure!(
        e.command.as_object().is_some_and(|o| o.keys().all(|k| [
            "story",
            "object",
            "participants",
            "fields",
            "about"
        ]
        .contains(&k.as_str()))),
        "Invalid scheduled command"
    );
    Ok(())
}

fn execute(
    state: &mut State,
    module: &str,
    body: Value,
    actor: &str,
    elapsed: u64,
    host: bool,
) -> Result<Value> {
    let mut c = state.connected.clone().unwrap();
    let mut body = body
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Command must be an object"))?;
    ensure!(
        body.keys().all(|k| [
            "command_id",
            "story",
            "object",
            "participants",
            "fields",
            "about"
        ]
        .contains(&k.as_str())),
        "Unknown command field (caller identity is host-owned)"
    );
    let id = body
        .remove("command_id")
        .and_then(|s| s.as_str().map(str::to_owned))
        .ok_or_else(|| anyhow::anyhow!("command_id required"))?;
    ensure!(
        !id.is_empty() && id.len() <= 128 && (host || !id.starts_with("host:")),
        "Invalid command id"
    );
    let fingerprint = json!({"module":module,"actor":actor,"command":body});
    if let Some(receipt) = c.receipts.get(&id) {
        ensure!(
            receipt["request"] == fingerprint,
            "Command ID reused with different inputs"
        );
        return Ok(receipt["result"].clone());
    }
    ensure!(c.receipts.len() < 500, "Institution receipt limit reached");
    let revision = c
        .modules
        .get(module)
        .ok_or_else(|| anyhow::anyhow!("Unknown module"))?
        .revision;
    body.insert("id".into(), json!(id));
    let response = evaluate(
        state,
        &c,
        elapsed,
        Evaluation {
            mode: "execute",
            module,
            actor,
            command: Some(json!(body)),
            migration: &[],
        },
    )?;
    let mut result = response["result"].clone();
    let delta = changes(&state.snapshot, &response["snapshot"]);
    let forbidden = delta.iter().any(|d| {
        let row = if d["after"].is_null() {
            &d["before"]
        } else {
            &d["after"]
        };
        let typ = row["type"].as_str().unwrap();
        !writable(&c, module, typ)
            && stored(&d["before"], &c.fields[typ]) != stored(&d["after"], &c.fields[typ])
    });
    if forbidden {
        result = json!({"committed":false,"violations":[{"kind":"write-scope","message":"Story attempted a stored write outside its module's grants"}]});
    }
    if !forbidden {
        c.standing = serde_json::from_value(response["standing"].clone())?;
        project(state, &c, &response)?;
    }
    result["module"] = json!(module);
    result["revision"] = json!(revision);
    result["command_id"] = json!(id);
    result["changes"] = if forbidden { json!([]) } else { json!(delta) };
    c.receipts.insert(id.clone(),json!({"request":fingerprint,"result":result,"revision":revision,"at":elapsed,"host_event":host}));
    c.provenance.push(json!({"kind":if host{"event"}else{"command"},"command_id":id,"module":module,"revision":revision,"at":elapsed,"actor":actor,"changes":result["changes"],"committed":result["committed"]}));
    state.connected = Some(c);
    Ok(result)
}
pub fn request(
    state: &mut State,
    method: &str,
    path: &str,
    body: Option<&str>,
    elapsed: u64,
) -> Result<Value> {
    let mut candidate = state.clone();
    let value = request_inner(&mut candidate, method, path, body, elapsed)?;
    *state = candidate;
    Ok(value)
}
fn request_inner(
    state: &mut State,
    method: &str,
    path: &str,
    body: Option<&str>,
    elapsed: u64,
) -> Result<Value> {
    if let Some((module, _)) = revision_path(path) {
        ensure!(method == "POST", "Amendments require POST");
        let target = format!("{}{}", ontology::ROOT.trim_end_matches('/'), path);
        let receipt = state
            .connected
            .as_ref()
            .unwrap()
            .revisions
            .iter()
            .find(|r| r["target"] == target)
            .ok_or_else(|| anyhow::anyhow!("Amendment has not been published"))?;
        ensure!(
            receipt["submitted"] == json!(body),
            "Amendment ID reused with different input"
        );
        return Ok(
            json!({"committed":true,"module":module,"revision":receipt["revision"],"receipt":receipt}),
        );
    }
    let (module, route) = if path.starts_with("/modules/") {
        let (name, route) = crate::institutions::module_path(path)
            .ok_or_else(|| anyhow::anyhow!("Unknown connected route"))?;
        ensure!(
            state.connected.as_ref().unwrap().modules.contains_key(name),
            "Unknown capability; open module root first"
        );
        (Some(name), route)
    } else {
        (None, path)
    };
    let c = state.connected.as_ref().unwrap();
    if method == "GET" && route == "/" {
        return Ok(
            json!({"actor":state.definition.actor,"modules":c.modules.keys().collect::<Vec<_>>(),"module":module,"revision":module.map(|m|c.modules[m].revision),"routes":["GET /model","GET /facts","GET /receipts","GET /events","GET /provenance","POST /commands"],"discovery":"Open /modules/NAME to access a capability. All capabilities share one live institution; reads include shared facts, writes obey type ownership and grants.","amendment":{"route":"POST /modules/NAME/revisions/ID","body":{"expected_revision":"current integer revision","mandate":"authorized structural change"},"authority":state.generation.as_ref().unwrap().connected.as_ref().unwrap().governors},"visibility":"Public facts"}),
        );
    }
    if method == "GET" && route == "/model" {
        return Ok(
            json!({"foundation":state.definition.source,"modules":c.modules,"owners":c.owners,"stories":state.stories,"scope":"One shared snapshot. Send exact story plus command_id, object/participants/fields to the owning module /commands. Module grants govern cross-module stored writes."}),
        );
    }
    if method == "GET" && route == "/facts" {
        return Ok(json!({"snapshot":state.snapshot,"owners":c.owners,"standing":c.standing}));
    }
    if method == "GET" && route == "/receipts" {
        return Ok(json!(c.receipts));
    }
    if method == "GET" && route == "/provenance" {
        return Ok(json!(c.provenance));
    }
    if method == "GET" && route == "/events" {
        return Ok(json!(
            c.modules
                .iter()
                .map(|(name, m)| (name, &m.events))
                .collect::<BTreeMap<_, _>>()
        ));
    }
    ensure!(
        method == "POST" && route == "/commands",
        "Unknown connected institution route"
    );
    let module = module.ok_or_else(|| anyhow::anyhow!("Execute a module's commands"))?;
    let actor = state.definition.actor.clone();
    execute(
        state,
        module,
        serde_json::from_str(body.unwrap_or("{}"))?,
        &actor,
        elapsed,
        false,
    )
}
pub fn next_event(state: &State) -> Option<u64> {
    state
        .connected
        .as_ref()?
        .modules
        .values()
        .flat_map(|m| m.events.iter())
        .filter(|e| e.remaining > 0)
        .map(|e| e.at)
        .min()
}
pub fn advance(state: &mut State, target: u64) -> Result<()> {
    let mut candidate = state.clone();
    let c = candidate.connected.as_ref().unwrap();
    ensure!(
        target >= c.elapsed,
        "Institution clock cannot move backward"
    );
    let mut count = 0;
    while let Some(at) = next_event(&candidate).filter(|t| *t <= target) {
        ensure!(
            count < 64,
            "Event catch-up limit exceeded; advance time in smaller steps"
        );
        count += 1;
        let c = candidate.connected.as_ref().unwrap();
        let (module, event) = c
            .modules
            .iter()
            .find_map(|(name, m)| {
                m.events
                    .iter()
                    .find(|e| e.remaining > 0 && e.at == at)
                    .map(|e| (name.clone(), e.clone()))
            })
            .unwrap();
        let rev = c.modules[&module].revision;
        let mut command = event.command.clone();
        command["command_id"] = json!(format!("host:{}:{}:{}:{}", module, rev, event.id, at));
        execute(&mut candidate, &module, command, &event.actor, at, true)?;
        let e = candidate
            .connected
            .as_mut()
            .unwrap()
            .modules
            .get_mut(&module)
            .unwrap()
            .events
            .iter_mut()
            .find(|e| e.id == event.id)
            .unwrap();
        e.remaining -= 1;
        if e.remaining > 0 {
            e.at =
                e.at.checked_add(e.every_seconds)
                    .ok_or_else(|| anyhow::anyhow!("Event clock overflow"))?;
        }
    }
    let mut c = candidate.connected.clone().unwrap();
    let response = evaluate(
        &candidate,
        &c,
        target,
        Evaluation {
            mode: "inspect",
            module: "",
            actor: &candidate.definition.actor,
            command: None,
            migration: &[],
        },
    )?;
    // Clock-derived facts may change; immutable foundation remains enforced.
    project(&mut candidate, &c, &response)?;
    c.standing = serde_json::from_value(response["standing"].clone())?;
    c.elapsed = target;
    candidate.connected = Some(c);
    *state = candidate;
    Ok(())
}

/// Keep ordinary world tools and add direct Story execution without making the
/// actor explicitly request generation or embedding commands in HTTP bodies.
pub fn configure_tools(world: &mut World) {
    if !world.ontology.as_ref().is_some_and(enabled) {
        return;
    }
    let tools = world.tools.as_mut().unwrap();
    let definitions = tools.definitions.as_array_mut().unwrap();
    for tool in crate::story_executor::definitions().as_array().unwrap() {
        if ["onto.inspect", "onto.execute"].contains(&tool["name"].as_str().unwrap())
            && !definitions.iter().any(|t| t["name"] == tool["name"])
        {
            definitions.push(tool.clone());
        }
    }
    tools.policy.push_str(" Institutions share one live snapshot. Use onto.inspect and onto.execute for direct model reads and Story commands; ordinary HTTP, search and mail remain available. The institution API documents amendment authority and revisions. Only committed receipts establish effects.");
}
