//! Optional public institution component. The trusted runtime is configured by
//! the host, never supplied as code by an actor or a construction proposal.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
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
    pub result: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct State {
    pub definition: Definition,
    pub runtime_sha256: String,
    pub snapshot: Value,
    pub stories: Value,
    pub receipts: BTreeMap<String, Receipt>,
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

fn runtime() -> Result<(Vec<u8>, String)> {
    let path = std::env::var("FRONTIER_ONTO_RUNTIME")
        .map_err(|_| anyhow::anyhow!("Optional Onto runtime is not configured"))?;
    let bytes = fs::read(&path).map_err(|e| {
        anyhow::anyhow!("Optional Onto runtime unavailable: {e}; run scripts/build-onto.mjs")
    })?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    Ok((bytes, digest))
}

fn bridge(input: Value, expected: &str) -> Result<Value> {
    let (bundle, digest) = runtime()?;
    ensure!(
        digest == expected,
        "Onto runtime changed; saved runs require their exact runtime bundle"
    );
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
    pub fn check_runtime(&self) -> Result<()> {
        ensure!(
            runtime()?.1 == self.runtime_sha256,
            "Onto runtime changed; saved runs require their exact runtime bundle"
        );
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
        let (_, runtime_sha256) = runtime()?;
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
        })
    }
    pub fn request(
        &mut self,
        method: &str,
        path: &str,
        body: Option<&str>,
        elapsed: u64,
    ) -> Result<Value> {
        if method == "GET" && path == "/" {
            return Ok(
                json!({"actor":self.definition.actor,"visibility":"All institution facts are public in this experiment.","routes":{"GET /model":"Canonical Onto source and available single actor Stories","GET /facts":"Current facts and derived values","POST /commands":{"command_id":"Stable unique key; reuse only for an identical retry","story":"Exact Story sentence","object":"Optional instance ID","participants":"Optional token to instance ID map","about":"Optional context ID","fields":"Optional creation fields; money is integer minor units"}},"authority":"Actor identity, source, clock, minting and system execution are host-owned."}),
            );
        }
        if method == "GET" && path == "/model" {
            return Ok(json!({"source":self.definition.source,"stories":self.stories}));
        }
        if method == "GET" && path == "/facts" {
            let response = bridge(
                json!({"operation":"inspect","source":self.definition.source,"actor":self.definition.actor,"epoch":self.definition.epoch,"elapsed_seconds":elapsed,"snapshot":self.snapshot}),
                &self.runtime_sha256,
            )?;
            self.snapshot = response["snapshot"].clone();
            return Ok(json!({"snapshot":self.snapshot,"standing":response["standing"]}));
        }
        ensure!(
            method == "POST" && path == "/commands",
            "Unknown institution route"
        );
        let input: Input = serde_json::from_str(body.unwrap_or("{}"))?;
        ensure!(
            !input.command_id.is_empty() && input.command_id.len() <= 128,
            "Invalid command_id"
        );
        let command = json!({"story":input.story,"object":input.object,"participants":input.participants,"about":input.about,"fields":input.fields});
        // Optional fields must be absent, not null, for the TypeScript API.
        let mut command = command.as_object().unwrap().clone();
        command.retain(|_, v| !v.is_null());
        let request_hash = format!("{:x}", Sha256::digest(serde_json::to_vec(&command)?));
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
            json!({"operation":"execute","source":self.definition.source,"actor":self.definition.actor,"epoch":self.definition.epoch,"elapsed_seconds":elapsed,"snapshot":self.snapshot,"command":command}),
            &self.runtime_sha256,
        )?;
        ensure!(
            serde_json::to_vec(&response["snapshot"])?.len() <= 262144,
            "Institution state exceeds 256 KiB"
        );
        self.snapshot = response["snapshot"].clone();
        let result = response["result"].clone();
        self.receipts.insert(
            input.command_id,
            Receipt {
                request_hash,
                result: result.clone(),
            },
        );
        Ok(result)
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
