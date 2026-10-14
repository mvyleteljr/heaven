//! Provider-compatible strict actor output, separate from persisted tool actions.
use anyhow::{Context, Result, ensure};
use frontier::{Action, tools};
use serde::Deserialize;
use serde_json::{Map, Value, json};

pub const STORY_INSTRUCTIONS: &str = r#"Return exactly {"action":{"kind":"TOOL_NAME","inputs":{...}}}, matching the selected tool's response schema. Supply every property shown; use null for an unused optional input. After reading the model and facts, execute a Story; repeated inspection does not test any rule. For onto.execute, copy the exact Story sentence and use actual instance IDs from the facts. participants_json and fields_json are JSON object strings, such as "{}", or null when unused. about is an instance ID only for a Story that declares an about context; it is NOT a description, rationale or test name. An approval command has this shape: {"action":{"kind":"onto.execute","inputs":{"module":"repair","command_id":"approval-probe-1","story":"Operator approves RepairRequest","object":"RepairRequest:intake","about":null,"participants_json":null,"fields_json":null}}}. Choose the actual test from the observed model and facts, not from this example. Check review_progress and use only currently available tools. After an input error, correct the identified fields; do not repeat the invalid call. Choose one action per response. End with finish and a nonempty message containing your evidence-based review."#;

pub const INSTRUCTIONS: &str = r#"Output transport v2: return exactly {"action":{"kind":"TOOL_NAME","inputs":{...}}}, matching the selected tool's schema in the response format. This transport overrides other response spelling in the tool policy. Supply every input property shown there; use null for an unused optional input. HTTP headers are an array of {"name":"Content-Type","value":"application/json"} entries, or null. HTTP body remains a string, including JSON-encoded request bodies. Do not emit multiple actions or call external tools. A GET example (replace the example URL with the intended service URL) is {"action":{"kind":"http.request","inputs":{"method":"GET","url":"https://service.example/resource","headers":null,"body":null}}}. To end the run, include your actual final report: {"action":{"kind":"finish","inputs":{"message":"Your report of confirmed results or remaining problems goes here."}}}. Never send empty inputs for finish or http.request. After an input error, correct the reported fields before retrying; do not repeat the same invalid call. A successful operation need not be submitted again to finish the run."#;

fn definition<'a>(definitions: &'a Value, name: &str) -> Result<&'a Value> {
    definitions
        .as_array()
        .and_then(|items| items.iter().find(|d| d["name"] == name))
        .ok_or_else(|| anyhow::anyhow!("Tool {name:?} is not enabled for this run"))
}

pub fn response_format(definitions: &Value) -> Result<Value> {
    let definitions = definitions.as_array().context("Invalid tool definitions")?;
    ensure!(!definitions.is_empty(), "Structured actions require tools");
    let mut branches = Vec::with_capacity(definitions.len());
    for definition in definitions {
        let name = definition["name"]
            .as_str()
            .context("Tool name is missing")?;
        let mut inputs = definition["input_schema"].clone();
        let required = inputs["required"]
            .as_array()
            .context("Missing required inputs")?
            .clone();
        let properties = inputs["properties"]
            .as_object_mut()
            .context("Missing tool properties")?;
        for (key, schema) in properties.iter_mut() {
            // Strict-output providers prohibit objects with arbitrary property names.
            // Translate the only map in tool inputs to typed pairs on the wire.
            if name == "http.request" && key == "headers" {
                *schema = json!({"type":"array","items":{"type":"object","properties":{"name":{"type":"string"},"value":{"type":"string"}},"required":["name","value"],"additionalProperties":false}});
            }
            if !required.contains(&json!(key)) {
                *schema = json!({"anyOf":[schema.clone(),{"type":"null"}]});
            }
        }
        // All properties must be required in strict mode; null means omitted internally.
        inputs["required"] = json!(properties.keys().collect::<Vec<_>>());
        inputs["additionalProperties"] = json!(false);
        branches.push(json!({"type":"object","description":definition["description"],"properties":{"kind":{"type":"string","enum":[name]},"inputs":inputs},"required":["kind","inputs"],"additionalProperties":false}));
    }
    // A nested union is supported by strict providers; a union at the root is not.
    Ok(
        json!({"type":"json_schema","json_schema":{"name":"actor_action_v2","strict":true,"schema":{"type":"object","properties":{"action":{"anyOf":branches}},"required":["action"],"additionalProperties":false}}}),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TypedAction {
    kind: String,
    inputs: Map<String, Value>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TypedEnvelope {
    action: TypedAction,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyEnvelope {
    kind: String,
    inputs_json: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    name: String,
    value: String,
}

pub fn parse(response: &str, definitions: &Value) -> Result<Action> {
    validated(decode_inner(response, definitions, false)?, definitions)
}

/// Decode only a known tool's envelope. Story review validates its arguments
/// during the recorded action, so correctable input errors become observations.
/// Malformed envelopes and unknown tools remain fatal protocol errors.
pub fn decode(response: &str, definitions: &Value) -> Result<Action> {
    decode_inner(response, definitions, true)
}

fn decode_inner(response: &str, definitions: &Value, recover_inputs: bool) -> Result<Action> {
    let value: Value = serde_json::from_str(response)?;
    if value.get("action").is_none() {
        // Old traces and saved runs keep their original transport. New requests
        // always advertise v2; valid legacy replies retain identical tool inputs.
        let envelope: LegacyEnvelope = serde_json::from_value(value)?;
        definition(definitions, &envelope.kind)?;
        ensure!(
            envelope.inputs_json.len() <= 65536,
            "Action inputs exceed 64 KiB"
        );
        let inputs: Value = serde_json::from_str(&envelope.inputs_json)?;
        ensure!(inputs.is_object(), "Action inputs must be a JSON object");
        return Ok(tools::call(&envelope.kind, inputs));
    }
    let envelope: TypedEnvelope = serde_json::from_value(value)?;
    let TypedAction { kind, mut inputs } = envelope.action;
    let definition = definition(definitions, &kind)?;
    let properties = definition["input_schema"]["properties"]
        .as_object()
        .context("Missing tool properties")?;
    let required = definition["input_schema"]["required"]
        .as_array()
        .context("Missing required inputs")?;
    for key in properties.keys() {
        ensure!(
            recover_inputs || inputs.contains_key(key),
            "Tool {kind} is missing input {key:?}; supply its value, or null only when optional"
        );
        if !required.contains(&json!(key)) && inputs.get(key).is_some_and(Value::is_null) {
            inputs.remove(key);
        }
    }
    if kind == "http.request"
        && let Some(headers) = inputs.remove("headers")
    {
        let headers: Vec<Header> = serde_json::from_value(headers)
            .context("HTTP headers must be an array of name/value pairs, or null")?;
        let mut map = Map::new();
        for header in headers {
            ensure!(
                !map.keys()
                    .any(|key: &String| key.eq_ignore_ascii_case(&header.name)),
                "Duplicate HTTP header {:?}",
                header.name
            );
            map.insert(header.name, json!(header.value));
        }
        inputs.insert("headers".into(), json!(map));
    }
    Ok(tools::call(&kind, json!(inputs)))
}

fn validated(action: Action, definitions: &Value) -> Result<Action> {
    tools::validate_action(&action)
        .map_err(|error| anyhow::anyhow!("{}", input_error(&action, definitions, &error)))?;
    Ok(action)
}

pub fn input_error(action: &Action, definitions: &Value, error: &anyhow::Error) -> String {
    let schema = definition(definitions, &action.kind)
        .map(|d| d["input_schema"].to_string())
        .unwrap_or_else(|_| "unknown tool".into());
    let example = match action.kind.as_str() {
        "finish" => {
            r#" Include message with your report of confirmed results or remaining problems; for example {"message":"I could not complete the task because …"}."#
        }
        "http.request" => {
            r#" Include method and url; for example {"method":"GET","url":"https://service.example/resource"}. Use the intended service URL, not this example URL. Encode a JSON request body as a string."#
        }
        "onto.execute" => {
            r#" Use null or "{}" for unused participants_json/fields_json; otherwise supply valid JSON object strings. object and about are instance IDs, never explanatory prose. Leave about null unless the selected Story declares an about context. Copy an exact Story sentence from the model."#
        }
        _ => "",
    };
    format!(
        "Invalid tool input for {}: {error}. Expected inputs schema: {schema}.{example} Correct the invalid fields before retrying; repeating the same inputs will fail again.",
        action.kind
    )
}
