//! Bounded, offline HTTP-shaped services. Each request launches a fresh isolated
//! BusyBox shell; only its workspace snapshot persists. No host daemon or network.
//!
//! Routes are `/services/{id}/{handler-path}`. The shell entrypoint receives
//! METHOD, handler-path, body-file as positional arguments, and REQUEST_METHOD,
//! REQUEST_PATH, REQUEST_BODY_FILE as environment variables. Read the body file
//! (not an interpolated shell string). Stdout must be one JSON object containing
//! an integer `status` (100..599) and `body` (any JSON value). Extra fields are
//! preserved but never interpreted as ledger operations.
//!
//! Probe checks GET /health => 200, {"status":"ok"} or {"working":true}, then
//! GET /entries (directory), /resources (coordination), or /orders (delivery).
//! Its body must contain a nonempty corresponding array of objects with nonblank
//! string `id` and `name` (entries/resources) or `item` (orders). Portable services
//! additionally need GET /export => 200, {"files":[{"path":...,"content":...}]}.
//! Evidence proves only these bounded public responses, not real-world usefulness,
//! truthful inventory, external delivery, or arbitrary actor claims.
use crate::workspace::{Workspace, path};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct State {
    services: BTreeMap<String, Service>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Service {
    entrypoint: String,
    capability: String,
    portable: bool,
    workspace: Workspace,
}

fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

impl State {
    pub fn ids(&self) -> Vec<String> {
        self.services.keys().cloned().collect()
    }
    /// Snapshot deployment. IDs are unique; deployment executes a health request
    /// to reject invalid execution/special-file snapshots, but need not be healthy.
    /// Returns {id,url,capability,portable}; portable here is only a declaration.
    pub fn deploy(&mut self, workspace: &Workspace, request: Value) -> Result<Value> {
        let id = request["id"].as_str().unwrap_or("");
        ensure!(
            !id.is_empty()
                && id.len() <= 64
                && id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
            "Invalid service id"
        );
        ensure!(
            !self.services.contains_key(id) && self.services.len() < 16,
            "Service already exists or hosting limit reached"
        );
        let entrypoint = path(request["entrypoint"].as_str().unwrap_or(""))?;
        ensure!(
            workspace.files.contains_key(&entrypoint),
            "Entrypoint must be a regular workspace file"
        );
        let capability = request["capability"].as_str().unwrap_or("");
        ensure!(
            ["directory", "coordination", "delivery"].contains(&capability),
            "Unsupported capability"
        );
        let portable = request["portable"]
            .as_bool()
            .ok_or_else(|| anyhow::anyhow!("portable must be boolean"))?;
        let mut service = Service {
            entrypoint,
            capability: capability.into(),
            portable,
            workspace: workspace.clone(),
        };
        service.execute("GET", "/health", None)?;
        self.services.insert(id.into(), service);
        Ok(
            json!({"id":id,"url":format!("/services/{id}"),"capability":capability,"portable":portable}),
        )
    }

    pub fn request(&mut self, method: &str, route: &str, body: Option<&str>) -> Result<Value> {
        let route = route
            .strip_prefix("/services/")
            .ok_or_else(|| anyhow::anyhow!("Expected /services/id/path"))?;
        let (id, suffix) = route.split_once('/').unwrap_or((route, ""));
        self.services
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Unknown service"))?
            .execute(method, &format!("/{suffix}"), body)
    }

    /// Independently runs public handlers. Execution/validation failure yields
    /// available:false, never inferred availability from deployment declarations.
    /// Return {id,capability,available,portable,evidence:{health,response,export}}.
    /// `portable` is verified, not the deployment flag. No ledger is mutated.
    pub fn probe(&mut self, id: &str) -> Result<Value> {
        let service = self
            .services
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Unknown service"))?;
        let (endpoint, key, label) = match service.capability.as_str() {
            "directory" => ("/entries", "entries", "name"),
            "coordination" => ("/resources", "resources", "name"),
            "delivery" => ("/orders", "orders", "item"),
            _ => anyhow::bail!("Unsupported capability"),
        };
        let health = service
            .execute("GET", "/health", None)
            .unwrap_or(Value::Null);
        let response = service
            .execute("GET", endpoint, None)
            .unwrap_or(Value::Null);
        let available = health["status"] == 200
            && (health["body"]["status"] == "ok" || health["body"]["working"] == true)
            && response["status"] == 200
            && response["body"][key].as_array().is_some_and(|a| {
                !a.is_empty() && a.iter().all(|v| nonblank(&v["id"]) && nonblank(&v[label]))
            });
        let export = if service.portable {
            service
                .execute("GET", "/export", None)
                .unwrap_or(Value::Null)
        } else {
            Value::Null
        };
        let portable = available
            && export["status"] == 200
            && export["body"]["files"].as_array().is_some_and(|a| {
                !a.is_empty()
                    && a.iter().all(|v| {
                        v["path"].as_str().is_some_and(|s| path(s).is_ok())
                            && nonblank(&v["content"])
                    })
            });
        Ok(
            json!({"id":id,"capability":service.capability,"available":available,"portable":portable,"evidence":{"health":health,"response":response,"export":export}}),
        )
    }
}

fn nonblank(v: &Value) -> bool {
    v.as_str().is_some_and(|s| !s.trim().is_empty())
}

impl Service {
    fn execute(&mut self, method: &str, route: &str, body: Option<&str>) -> Result<Value> {
        ensure!(
            !method.is_empty()
                && method.len() <= 16
                && method.bytes().all(|b| b.is_ascii_uppercase()),
            "Invalid method"
        );
        ensure!(
            route.starts_with('/') && route.len() <= 2048 && !route.contains('\0'),
            "Invalid request path"
        );
        let mut next = self.workspace.clone();
        // Reserve a fresh temporary name outside the persistent workspace.
        let bodyfile = "/tmp/hosting-body";
        ensure!(
            body.unwrap_or("").len() <= 8192 && !body.unwrap_or("").contains('\0'),
            "Body too large or contains NUL"
        );
        let command = format!(
            "printf '%s' {} > {bodyfile}; REQUEST_METHOD={} REQUEST_PATH={} REQUEST_BODY_FILE={bodyfile} /bin/sh {} {} {} {bodyfile}",
            quote(body.unwrap_or("")),
            quote(method),
            quote(route),
            quote(&format!("/workspace/{}", self.entrypoint)),
            quote(method),
            quote(route)
        );
        let output = next.exec(&command)?;
        ensure!(
            output["exit_status"] == 0
                && output["truncated"] == false
                && output["timed_out"] == false,
            "Handler execution failed: {}",
            output["stderr"]
        );
        let response: Value = serde_json::from_str(output["stdout"].as_str().unwrap_or(""))?;
        ensure!(
            response.is_object()
                && response["status"]
                    .as_u64()
                    .is_some_and(|s| (100..=599).contains(&s))
                && response.get("body").is_some(),
            "Handler must return JSON status and body"
        );
        self.workspace = next;
        Ok(response)
    }
}
