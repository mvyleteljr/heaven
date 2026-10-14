//! Optional public institution component. The trusted runtime is configured by
//! the host, never supplied as code by an actor or a construction proposal.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

pub const ROOT: &str = "https://institutions.local/";
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub source: BTreeMap<String, String>,
    pub actor: String,
    pub epoch: String,
    /// V1 deliberately exposes all model facts; private/role-filtered reads need
    /// a separate adapter and cannot be enabled by merely omitting this flag.
    pub public_facts: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Receipt {
    pub request_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submitted_sha256: Option<String>,
    pub result: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct State {
    pub definition: Definition,
    pub runtime_sha256: String,
    pub snapshot: Value,
    pub stories: Value,
    pub receipts: BTreeMap<String, Receipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation: Option<crate::institutions::Policy>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub modules: BTreeMap<String, State>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protected: Vec<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protected_types: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    command_id: String,
    story: String,
    object: Option<String>,
    participants: Option<BTreeMap<String, String>>,
    about: Option<String>,
    fields: Option<BTreeMap<String, Value>>,
}

fn runtime(expected: Option<&str>) -> Result<(Vec<u8>, String)> {
    let path = std::env::var("FRONTIER_ONTO_RUNTIME")
        .map_err(|_| anyhow::anyhow!("Optional Onto runtime is not configured"))?;
    runtime_at(Path::new(&path), expected)
}

fn runtime_at(path: &Path, expected: Option<&str>) -> Result<(Vec<u8>, String)> {
    // The configured default selects new runs. A saved run may use only its
    // exact content-addressed bundle, never a newer interpreter by accident.
    if let Some(expected) = expected {
        ensure!(
            expected.len() == 64 && expected.bytes().all(|b| b.is_ascii_hexdigit()),
            "Invalid saved Onto runtime hash"
        );
        if let Ok(bytes) = fs::read(path) {
            let digest = format!("{:x}", Sha256::digest(&bytes));
            if digest == expected {
                return Ok((bytes, digest));
            }
        }
        let archive = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("onto-runtimes")
            .join(format!("{expected}.cjs"));
        let bytes = fs::read(&archive).map_err(|e| {
            anyhow::anyhow!(
                "Saved Onto runtime {expected} is unavailable at {}: {e}; restore the original bundle",
                archive.display()
            )
        })?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        ensure!(
            digest == expected,
            "Archived Onto runtime hash mismatch; saved runs require their exact runtime bundle"
        );
        return Ok((bytes, digest));
    }
    let bytes = fs::read(path).map_err(|e| {
        anyhow::anyhow!("Optional Onto runtime unavailable: {e}; run scripts/build-onto.mjs")
    })?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    Ok((bytes, digest))
}

pub(crate) fn bridge(input: Value, expected: &str) -> Result<Value> {
    let (bundle, _) = runtime(Some(expected))?;
    let request = serde_json::to_vec(&input)?;
    ensure!(request.len() <= 524_288, "Onto request exceeds 512 KiB");
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("runtime.cjs");
    fs::write(&path, bundle)?;
    fs::write(dir.path().join("input.json"), request)?;
    // File-backed I/O avoids pipe deadlocks; poll and kill bound execution and output.
    let output = dir.path().join("output.json");
    let errors = dir.path().join("errors.txt");
    let mut child = Command::new("node")
        .arg("--max-old-space-size=128")
        .arg(path)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .stdin(fs::File::open(dir.path().join("input.json"))?)
        .stdout(fs::File::create(&output)?)
        .stderr(fs::File::create(&errors)?)
        .spawn()?;
    let started = Instant::now();
    let status = loop {
        if started.elapsed() > Duration::from_secs(5)
            || fs::metadata(&output)?.len() > 1_048_576
            || fs::metadata(&errors)?.len() > 32768
        {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("Onto runtime execution limit reached");
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    ensure!(
        fs::metadata(&output)?.len() <= 1_048_576,
        "Onto response too large"
    );
    let response: Value = serde_json::from_slice(&fs::read(output)?)?;
    ensure!(
        status.success() && response.get("error").is_none(),
        "Onto refused: {}",
        response["error"]
    );
    Ok(response)
}

impl State {
    fn command_guide(&self, base: &str) -> Value {
        let mut guide = json!({
            "endpoint":format!("{base}/commands"),
            "method":"POST",
            "instructions":[
                "Use the absolute endpoint above. There is no /api prefix.",
                format!("Copy a story string from GET {base}/model verbatim. Keep tokens such as Account:sender in the Story sentence; do not replace them with account names or append amounts."),
                format!("Bind each participant token to an instance ID from GET {base}/facts in participants. Put creation values such as amount and reference in fields."),
                "Send the command as a JSON-encoded string in http.request inputs.body, with method POST and the absolute url. command_id is a required unique key. Reuse it only for an identical retry; a retry must not create a second effect.",
                "A 400 response means the request was invalid. Correct the route or payload using the error before trying again. A successful HTTP response alone does not establish success: inspect committed and violations, then verify the facts."
            ]
        });
        // Keep this example out of unrelated authored models. It illustrates the
        // wire format; the runtime still validates the current model and facts.
        let story = "Clerk creates Transfer from Account:sender to Account:recipient";
        let accounts = self.stories.as_array().is_some_and(|stories| {
            stories
                .iter()
                .any(|s| s["story"] == story && s["creates"] == true)
        }) && ["Account:Operating", "Account:Reserve"].iter().all(|id| {
            self.snapshot["instances"]
                .as_array()
                .is_some_and(|instances| {
                    instances
                        .iter()
                        .any(|i| i["id"] == *id && i["type"] == "Account")
                })
        });
        if accounts {
            let command = json!({
                "command_id":"example-transfer-1",
                "story":story,
                "participants":{"Account:sender":"Account:Operating","Account:recipient":"Account:Reserve"},
                "fields":{"amount":2500,"reference":"example-transfer-1"}
            });
            guide["example_note"] = json!(
                "Example for a $25 Operating-to-Reserve transfer. Use it only when it matches your task and current model. Check existing transfers before sending; use a new key/reference for a distinct authorized transfer."
            );
            guide["example_action"] = json!({
                "kind":"http.request",
                "inputs":{"method":"POST","url":format!("{base}/commands"),"headers":{"content-type":"application/json"},"body":command.to_string()}
            });
        }
        guide
    }

    pub fn check_runtime(&self) -> Result<()> {
        let mut checked = BTreeSet::new();
        let mut pending = vec![self];
        while let Some(state) = pending.pop() {
            if checked.insert(&state.runtime_sha256) {
                runtime(Some(&state.runtime_sha256))?;
            }
            pending.extend(state.modules.values());
        }
        Ok(())
    }
    pub fn initialize(definition: Definition) -> Result<Self> {
        ensure!(
            definition.public_facts,
            "Onto v1 requires explicitly public model facts"
        );
        ensure!(
            !definition.source.is_empty() && definition.source.len() <= 16,
            "Provide 1–16 Onto source files"
        );
        ensure!(
            definition.source.iter().all(|(k, v)| !k.contains("..")
                && !k.starts_with('/')
                && k.ends_with(".onto")
                && k.len() <= 100
                && v.len() <= 65536),
            "Invalid Onto source files"
        );
        ensure!(
            definition.actor.len() <= 200 && definition.epoch.len() <= 40,
            "Invalid Onto identity/epoch"
        );
        let (_, runtime_sha256) = runtime(None)?;
        let response = bridge(
            json!({"operation":"initialize","source":definition.source,"actor":definition.actor,"epoch":definition.epoch,"elapsed_seconds":0}),
            &runtime_sha256,
        )?;
        let mut definition = definition;
        definition.source = serde_json::from_value(response["source"].clone())?;
        Ok(Self {
            definition,
            runtime_sha256,
            snapshot: response["snapshot"].clone(),
            stories: response["stories"].clone(),
            receipts: BTreeMap::new(),
            generation: None,
            modules: BTreeMap::new(),
            protected: vec![],
            protected_types: vec![],
        })
    }
    pub fn request(
        &mut self,
        method: &str,
        path: &str,
        body: Option<&str>,
        elapsed: u64,
    ) -> Result<Value> {
        self.request_at(method, path, body, elapsed, ROOT.trim_end_matches('/'))
    }

    fn request_at(
        &mut self,
        method: &str,
        path: &str,
        body: Option<&str>,
        elapsed: u64,
        base: &str,
    ) -> Result<Value> {
        if let Some(rest) = path.strip_prefix("/modules/") {
            let key = rest.split('/').next().unwrap_or_default();
            if let Some(module) = self.modules.get_mut(key) {
                // Keep an invalid suffix too so errors point back to this
                // module's endpoints, rather than the read-only foundation.
                let suffix = &rest[key.len()..];
                let route = if suffix.is_empty() { "/" } else { suffix };
                return module.request_at(
                    method,
                    route,
                    body,
                    elapsed,
                    &format!("{base}/modules/{key}"),
                );
            }
            if crate::institutions::module_path(path).is_some() {
                anyhow::bail!(
                    "Capability unavailable at this route; open {base}/modules/NAME first"
                );
            }
        }
        if method == "GET" && path == "/" {
            let mut document = json!({
                "actor":self.definition.actor,
                "visibility":"All institution facts are public in this experiment.",
                "urls":{"api":if base == ROOT.trim_end_matches('/') { ROOT } else { base },"model":format!("{base}/model"),"facts":format!("{base}/facts")},
                "routes":{"GET /model":"Canonical Onto source and available single actor Stories","GET /facts":"Current facts and derived values"},
                "authority":"Actor identity, source, foundation, clock, minting and system execution are host-owned."
            });
            if self.generation.is_some() {
                document["module_discovery"] = json!(format!(
                    "Browse a business capability at GET {base}/modules/NAME. NAME is 1-48 lowercase ASCII letters, digits or hyphens. Open the capability before using its /model, /facts and /commands routes. Modules share a read-only company foundation; operational state is local to each module."
                ));
            } else {
                document["urls"]["commands"] = json!(format!("{base}/commands"));
                document["routes"]["POST /commands"] = json!({"command_id":"Required stable unique key; reuse only for an identical retry","story":"Required exact Story sentence copied verbatim from /model","object":"Optional instance ID","participants":"Token to instance ID map; required when the Story has participants","about":"Optional context ID","fields":"Creation fields from the model; money is integer minor units"});
                document["command_guide"] = self.command_guide(base);
            }
            return Ok(document);
        }
        if method == "GET" && path == "/model" {
            let mut document = json!({"source":self.definition.source,"stories":self.stories});
            if self.generation.is_none() {
                document["command_guide"] = self.command_guide(base);
            }
            return Ok(document);
        }
        if method == "GET" && path == "/facts" {
            let response = bridge(
                json!({"operation":"inspect","source":self.definition.source,"actor":self.definition.actor,"epoch":self.definition.epoch,"elapsed_seconds":elapsed,"snapshot":self.snapshot}),
                &self.runtime_sha256,
            )?;
            self.check_protected(&response["snapshot"])?;
            self.snapshot = response["snapshot"].clone();
            return Ok(json!({"snapshot":self.snapshot,"standing":response["standing"]}));
        }
        ensure!(
            method == "POST" && path == "/commands",
            "Unknown institution route: {method} {path}. Use GET {base}/ for documentation, GET {base}/model, GET {base}/facts, or POST {base}/commands. There is no /api prefix. The documentation includes a complete command example when available."
        );
        ensure!(
            self.generation.is_none(),
            "Company foundation is read-only; execute a module's commands"
        );
        let input: Input = serde_json::from_str(body.unwrap_or("{}")).map_err(|error| {
            anyhow::anyhow!("Invalid institution command: {error}. POST {base}/commands with a JSON object containing required command_id and exact story, participants mapping Story tokens to instance IDs, and fields for creation values. For http.request, put this JSON in inputs.body as a string. Read GET {base}/model for the exact Stories and command guide.")
        })?;
        ensure!(
            !input.command_id.is_empty() && input.command_id.len() <= 128,
            "Invalid command_id"
        );
        ensure!(
            !input.command_id.starts_with("host:"),
            "Reserved host command ID"
        );
        ensure!(
            self.stories
                .as_array()
                .is_some_and(|stories| stories.iter().any(|s| s["story"] == input.story)),
            "Unknown Story. Copy one of these exact Story sentences verbatim: {}. Bind instance IDs in participants and creation values in fields; do not rewrite the Story or append values. See GET {base}/model for the command guide.",
            json!(
                self.stories
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|s| &s["story"])
                    .collect::<Vec<_>>()
            )
        );
        let actor = self.definition.actor.clone();
        self.command(input, elapsed, &actor, false)
    }
    /// Trusted scheduling/probe adapter only. Never accept a caller identity from an actor tool.
    pub(crate) fn host_command(&mut self, actor: &str, body: Value, elapsed: u64) -> Result<Value> {
        let input: Input = serde_json::from_value(body)?;
        ensure!(
            input.command_id.starts_with("host:") && input.command_id.len() <= 128,
            "Expected a reserved host command ID"
        );
        self.command(input, elapsed, actor, true)
    }
    fn command(&mut self, input: Input, elapsed: u64, actor: &str, trusted: bool) -> Result<Value> {
        let command = json!({"story":input.story,"object":input.object,"participants":input.participants,"about":input.about,"fields":input.fields});
        // Optional fields must be absent, not null, for the TypeScript API.
        let mut command = command.as_object().unwrap().clone();
        command.retain(|_, v| !v.is_null());
        let hash_input = if trusted {
            json!({"actor":actor,"command":command})
        } else {
            json!(command)
        };
        let request_hash = format!("{:x}", Sha256::digest(serde_json::to_vec(&hash_input)?));
        if let Some(receipt) = self.receipts.get(&input.command_id) {
            ensure!(
                receipt.request_hash == request_hash,
                "command_id already belongs to a different request"
            );
            return Ok(receipt.result.clone());
        }
        ensure!(
            self.receipts.len() < 500,
            "Institution command limit reached"
        );
        command.insert("id".into(), json!(input.command_id));
        let response = bridge(
            json!({"operation":"execute","source":self.definition.source,"actor":actor,"epoch":self.definition.epoch,"elapsed_seconds":elapsed,"snapshot":self.snapshot,"command":command}),
            &self.runtime_sha256,
        )?;
        ensure!(
            serde_json::to_vec(&response["snapshot"])?.len() <= 262144,
            "Institution state exceeds 256 KiB"
        );
        self.check_protected(&response["snapshot"])?;
        self.snapshot = response["snapshot"].clone();
        let result = response["result"].clone();
        self.receipts.insert(
            input.command_id,
            Receipt {
                request_hash,
                submitted_sha256: None,
                result: result.clone(),
            },
        );
        Ok(result)
    }

    fn check_protected(&self, snapshot: &Value) -> Result<()> {
        let actual: Vec<_> = snapshot["instances"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid snapshot"))?
            .iter()
            .filter(|v| self.protected_types.iter().any(|t| v["type"] == *t))
            .cloned()
            .collect();
        ensure!(
            actual == self.protected,
            "Generated command would change protected foundation facts or identity"
        );
        Ok(())
    }
}

pub fn route(url: &str) -> Option<String> {
    let url = reqwest::Url::parse(url).ok()?;
    (url.scheme() == "https"
        && url.host_str() == Some("institutions.local")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none())
    .then(|| url.path().to_owned())
}
pub fn http(world: &mut crate::World, action: &crate::Action) -> Result<Option<Value>> {
    let input = action.inputs.as_ref().unwrap();
    let Some(path) = input["url"].as_str().and_then(route) else {
        return Ok(None);
    };
    let Some(state) = &mut world.ontology else {
        return Ok(None);
    };
    let elapsed = world.tools.as_ref().unwrap().elapsed_seconds;
    let mut candidate = state.clone();
    let result = candidate.request(
        &input["method"].as_str().unwrap().to_uppercase(),
        &path,
        input["body"].as_str(),
        elapsed,
    );
    let (status, body) = match result {
        Ok(result) => {
            *state = candidate;
            (200, result)
        }
        Err(error) => (400, json!({"error":error.to_string()})),
    };
    Ok(Some(
        json!({"status":status,"headers":{"content-type":"application/json"},"body":body.to_string(),"truncated":false}),
    ))
}
