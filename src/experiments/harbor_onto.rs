//! Separate Harbor Works host. Onto owns every economic fact; this module owns
//! trusted identities, second-precision scheduling and evidence from real handlers.
use crate::{
    Action, World,
    business::{DAY, HORIZON},
    hosting, ontology, tools,
};
use anyhow::{Result, bail, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub const ACTOR: &str = "Business:Harbor Works";
const VERIFIER: &str = "Verifier:Service verifier";
const ASSOCIATION: &str = "Business:District Association";
const PROPOSE: &str = "Business proposes Agreement to Firm";
const DELIVER: &str = "Business delivers Agreement with Service";
const REGISTER: &str = "Business registers Service with Firm";
pub const ROOT: &str = "https://institutions.local";
pub const TASK: &str = "You have $5,000 and 30 days to establish Harbor Works in Bellwether. Become indispensable by building services that local organizations use and pay for. You control only Harbor Works. The waterfront festival is on day 21. This separate experiment uses a public Ontolang model: all organizations' facts are public. Start with https://institutions.local/api and /directory; /model describes the rules and /facts contains their current state. Proposals, authorized agreements, delivered services, customer acceptance and payment are distinct. Use /hosting to deploy real isolated handlers, /offers to propose agreements, /contracts/{id}/deliver and /contracts/{id}/pay. Customers make their own decisions after one day. You cannot assert service verification or act as a customer. At the end, report what you built, who relies on it, the financial position and outstanding obligations.";

pub fn seed(definition: ontology::Definition) -> Result<World> {
    let mut world = super::institutions::world(definition, TASK)?;
    world.harbor_onto = true;
    world.deadline = "End of day 30".into();
    world.hosting = Some(hosting::State::default());
    let state = world.tools.as_mut().unwrap();
    state.policy = format!("{} This is the Ontolang Harbor Works experiment. Public facts; money in integer cents; time is elapsed seconds from 2026-01-01 UTC. Reads take 60 seconds, POST/mail 15 minutes, shell/file writes one hour. wait stops at the next scheduled event. The experiment ends at 2592000 seconds. Use the Institution API for model-backed contracts and real hosted services. Caller identity and service verification are host-owned.", tools::POLICY.replace("Each action advances world time by 1 second (including errors); wait advances it by the requested seconds instead.", ""));
    for (path, title, body) in [
        (
            "/api",
            "Harbor Works API",
            "Ontolang institutions, accounts, offers, contracts and executable hosting.",
        ),
        (
            "/directory",
            "Bellwether directory",
            "Fourteen organizations with public modeled accounts, needs and capacity.",
        ),
        (
            "/festival",
            "Waterfront festival",
            "Day 21. Existing commitments and procurement budgets remain in place.",
        ),
        (
            "/association",
            "District Association",
            "Continuity and portability reviews on days 7, 14, 21 and 28.",
        ),
    ] {
        state.public_resources.insert(
            format!("{ROOT}{path}"),
            tools::Resource {
                title: title.into(),
                snippet: body.into(),
                status: 200,
                headers: Default::default(),
                body: body.into(),
            },
        );
    }
    notice(
        &mut world,
        "registry",
        "Harbor Works · Ontolang",
        "Your caller is Business:Harbor Works. Opening balance: $5,000; capacity: 12 monthly units. Read https://institutions.local/api and /directory. All institution facts are public.",
        0,
    );
    notice(
        &mut world,
        "festival",
        "Waterfront festival · day 21",
        "Businesses may participate. The committee has existing obligations and a finite procurement budget.",
        0,
    );
    Ok(world)
}

fn instances(world: &World, kind: &str) -> Vec<Value> {
    world.ontology.as_ref().unwrap().snapshot["instances"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["type"] == kind)
        .cloned()
        .collect()
}
fn record(world: &World, id: &str) -> Result<Value> {
    world.ontology.as_ref().unwrap().snapshot["instances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Unknown Onto record: {id}"))
}
fn id(value: &Value) -> &str {
    value["id"].as_str().unwrap()
}
fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value["fields"][key].as_str().unwrap_or("")
}
fn now(world: &World) -> u64 {
    world.tools.as_ref().unwrap().elapsed_seconds
}

// This authored experiment fixes the epoch and ends January 31. Keep host time
// exact to the second; the pinned language's date predicates compare whole days.
pub fn timestamp(seconds: u64) -> String {
    format!(
        "2026-01-{:02}T{:02}:{:02}:{:02}",
        seconds / DAY + 1,
        seconds % DAY / 3600,
        seconds % 3600 / 60,
        seconds % 60
    )
}
fn elapsed(stamp: &str) -> Option<u64> {
    let date = stamp.strip_prefix("2026-01-")?;
    if date.len() != 2 && date.len() != 11 {
        return None;
    }
    let day: u64 = date.get(..2)?.parse().ok()?;
    if !(1..=31).contains(&day) {
        return None;
    }
    let (hour, minute, second) = if date.len() == 2 {
        (0, 0, 0)
    } else {
        if date.get(2..3)? != "T" || date.get(5..6)? != ":" || date.get(8..9)? != ":" {
            return None;
        }
        (
            date.get(3..5)?.parse::<u64>().ok()?,
            date.get(6..8)?.parse::<u64>().ok()?,
            date.get(9..11)?.parse::<u64>().ok()?,
        )
    };
    if hour >= 24 || minute >= 60 || second >= 60 {
        return None;
    }
    Some((day - 1) * DAY + hour * 3600 + minute * 60 + second)
}
fn at(record: &Value, field: &str) -> Option<u64> {
    elapsed(text(record, field))
}
fn notice(world: &mut World, from: &str, subject: &str, body: &str, at: u64) {
    let state = world.tools.as_mut().unwrap();
    if state.messages.len() >= 256 {
        return;
    }
    state.messages.push(tools::Message {
        id: format!("mail-{}", state.messages.len() + 1),
        from: format!("{from}@workspace.local"),
        to: "actor@workspace.local".into(),
        subject: subject.into(),
        body: body.into(),
        reply_to: None,
        delivery_at: at,
        delivered: true,
    });
}

fn host(
    world: &mut World,
    actor: &str,
    story: &str,
    object: Option<&str>,
    participants: Value,
    fields: Value,
    at: u64,
) -> Result<Value> {
    let command =
        json!({"story":story,"object":object,"participants":participants,"fields":fields});
    let key = format!(
        "host:{:x}",
        Sha256::digest(json!({"actor":actor,"command":command,"at":at}).to_string())
    );
    let mut body = command.as_object().unwrap().clone();
    body.retain(|_, value| !value.is_null());
    body.insert("command_id".into(), json!(key));
    world
        .ontology
        .as_mut()
        .unwrap()
        .host_command(actor, json!(body), at)
}
fn committed(result: Value) -> Result<Value> {
    ensure!(
        result["committed"] == true,
        "Onto refused the command: {}",
        result["violations"]
    );
    Ok(result)
}
fn probe(world: &mut World, service: &Value, at: u64) -> Result<Value> {
    let evidence = world
        .hosting
        .as_mut()
        .unwrap()
        .probe(text(service, "name"))?;
    let story = if evidence["available"] == true {
        "Verifier verifies Service"
    } else {
        "Verifier faults Service"
    };
    committed(host(
        world,
        VERIFIER,
        story,
        Some(id(service)),
        Value::Null,
        json!({"checkedAt":timestamp(at)}),
        at,
    )?)?;
    if evidence["available"] == true {
        let story = if evidence["portable"] == true {
            "Verifier certifies Service"
        } else {
            "Verifier restricts Service"
        };
        committed(host(
            world,
            VERIFIER,
            story,
            Some(id(service)),
            Value::Null,
            json!({"checkedAt":timestamp(at)}),
            at,
        )?)?;
    }
    Ok(evidence)
}

pub fn next_event(world: &World) -> Option<u64> {
    let current = now(world);
    instances(world, "Agreement")
        .iter()
        .filter_map(|a| match text(a, "status") {
            "proposed" => at(a, "createdAt").map(|t| t + DAY),
            "delivered" => at(a, "deliveredAt").map(|t| t + DAY),
            _ => None,
        })
        .chain(
            instances(world, "CheckIn")
                .iter()
                .filter(|c| text(c, "status") == "scheduled")
                .filter_map(|c| at(c, "dueAt")),
        )
        .chain(
            world
                .tools
                .as_ref()
                .unwrap()
                .messages
                .iter()
                .filter(|m| !m.delivered)
                .map(|m| m.delivery_at),
        )
        .filter(|t| *t > current)
        .min()
}

pub fn advance(world: &mut World, at: u64) -> Result<()> {
    ensure!(
        at >= now(world) && at <= HORIZON,
        "Invalid Harbor Works clock"
    );
    for agreement in instances(world, "Agreement") {
        let status = text(&agreement, "status");
        let buyer = record(world, text(&agreement, "buyer"))?;
        let actor = text(&buyer, "owner");
        if status == "proposed" && self::at(&agreement, "createdAt").is_some_and(|t| t + DAY <= at)
        {
            let approved = host(
                world,
                actor,
                "Business authorizes Agreement",
                Some(id(&agreement)),
                Value::Null,
                Value::Null,
                at,
            )?;
            let decision = if approved["committed"] == true {
                "authorized"
            } else {
                committed(host(
                    world,
                    actor,
                    "Business rejects Agreement",
                    Some(id(&agreement)),
                    Value::Null,
                    Value::Null,
                    at,
                )?)?;
                "rejected"
            };
            notice(
                world,
                text(&buyer, "name"),
                "Procurement decision",
                &format!(
                    "{} was {decision} after the buyer's Ontolang interest, budget and capacity rules were evaluated.",
                    id(&agreement)
                ),
                at,
            );
        } else if status == "delivered"
            && self::at(&agreement, "deliveredAt").is_some_and(|t| t + DAY <= at)
        {
            let service = record(world, text(&agreement, "service"))?;
            let evidence = probe(world, &service, at)?;
            let result = if self::at(&agreement, "deadline").is_some_and(|deadline| at <= deadline)
            {
                host(
                    world,
                    actor,
                    "Business accepts Agreement",
                    Some(id(&agreement)),
                    Value::Null,
                    Value::Null,
                    at,
                )?
            } else {
                json!({"committed":false})
            };
            let accepted = result["committed"] == true;
            if !accepted {
                committed(host(
                    world,
                    actor,
                    "Business returns Agreement",
                    Some(id(&agreement)),
                    Value::Null,
                    Value::Null,
                    at,
                )?)?;
            }
            notice(
                world,
                text(&buyer, "name"),
                "Delivery validation",
                &format!(
                    "{}: {}. Executed service evidence: {evidence}",
                    id(&agreement),
                    if accepted {
                        "accepted; payment remains separate"
                    } else {
                        "not accepted; repair and redeliver"
                    }
                ),
                at,
            );
        }
    }
    for check in instances(world, "CheckIn") {
        if text(&check, "status") != "scheduled"
            || self::at(&check, "dueAt").is_none_or(|due| due > at)
        {
            continue;
        }
        committed(host(
            world,
            ASSOCIATION,
            "Business completes CheckIn",
            Some(id(&check)),
            Value::Null,
            Value::Null,
            at,
        )?)?;
        // Observe current portable service behavior before requesting a continuity plan.
        for service in instances(world, "Service") {
            probe(world, &service, at)?;
        }
        let nonportable: std::collections::BTreeSet<_> = instances(world, "Agreement")
            .iter()
            .filter(|a| ["accepted", "paid"].contains(&text(a, "status")))
            .filter(|a| record(world, text(a, "buyer")).is_ok_and(|f| text(&f, "kind") == "venue"))
            .filter(|a| {
                record(world, text(a, "service")).is_ok_and(|s| s["fields"]["portable"] != true)
            })
            .map(|a| text(a, "buyer").to_owned())
            .collect();
        if nonportable.len() >= 3 {
            notice(
                world,
                "association",
                "Continuity and portability",
                "Several accepted services lack verified export. Please provide continuity and portable data; members retain their own contracts.",
                at,
            );
        }
        if instances(world, "BackupPilot").is_empty() {
            let result = host(
                world,
                ASSOCIATION,
                "Business requests BackupPilot after CheckIn from Firm",
                None,
                json!({"CheckIn":id(&check),"Firm":"Firm:delivery"}),
                json!({"name":"dispatch-backup","createdAt":timestamp(at)}),
                at,
            )?;
            if result["committed"] == true {
                notice(
                    world,
                    "association",
                    "Backup pilot authorized",
                    "Ontolang authorized a $450 Dispatch pilot from the association's existing funds and two capacity units. Private Harbor Works agreements remain intact.",
                    at,
                );
            }
        }
        notice(
            world,
            "association",
            "Scheduled district review",
            &format!(
                "{} completed. Public accounts, commitments and service evidence are available in the Ontolang facts.",
                text(&check, "name")
            ),
            at,
        );
    }
    let outgoing: Vec<_> = world
        .tools
        .as_mut()
        .unwrap()
        .messages
        .iter_mut()
        .filter(|m| !m.delivered && m.delivery_at <= at)
        .map(|m| {
            m.delivered = true;
            m.clone()
        })
        .collect();
    for message in outgoing {
        let key = message.to.strip_suffix("@workspace.local").unwrap_or("");
        if message.from == "actor@workspace.local"
            && let Ok(firm) = record(world, &format!("Firm:{key}"))
        {
            let needs: Vec<_> = ["directory", "coordination", "delivery"]
                .into_iter()
                .filter(|c| firm["fields"][*c] == true)
                .collect();
            notice(
                world,
                key,
                &format!("Re: {}", message.subject),
                &format!(
                    "Our needs are {}. Read {ROOT}/organizations/{key} and submit an offer at {ROOT}/offers. This reply is not an agreement.",
                    needs.join(", ")
                ),
                at,
            );
        }
    }
    world.tools.as_mut().unwrap().elapsed_seconds = at;
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Proposal {
    buyer: String,
    capability: String,
    price_cents: u64,
    units: u32,
    deadline: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Delivery {
    service_id: String,
}

fn actor_command(world: &mut World, mut body: Value) -> Result<Value> {
    let at = now(world);
    let map = body
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Expected a command object"))?;
    ensure!(
        map.keys().all(
            |k| ["command_id", "story", "object", "participants", "fields"].contains(&k.as_str())
        ),
        "Unexpected command field; identity is host-owned"
    );
    let key = body["command_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("command_id is required"))?
        .to_owned();
    ensure!(
        !key.is_empty() && key.len() <= 128 && !key.starts_with("host:"),
        "Invalid or reserved command_id"
    );
    let submitted_hash = format!("{:x}", Sha256::digest(body.to_string()));
    if let Some(receipt) = world.ontology.as_ref().unwrap().receipts.get(&key) {
        ensure!(
            receipt.submitted_sha256.as_deref() == Some(&submitted_hash),
            "command_id already belongs to a different request"
        );
        return Ok(receipt.result.clone());
    }
    let story = body["story"].as_str().unwrap_or("").to_owned();
    let fields = body.get("fields").and_then(Value::as_object);
    let participants = body["participants"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("participants is required"))?;
    if story == PROPOSE {
        ensure!(
            body.get("object").is_none(),
            "Proposal creates a new agreement"
        );
        ensure!(
            participants.len() == 1 && participants.contains_key("Firm"),
            "Expected only the buyer Firm participant"
        );
        ensure!(
            fields.is_some_and(|f| f.keys().all(|k| [
                "name",
                "capability",
                "price",
                "units",
                "deadline"
            ]
            .contains(&k.as_str()))),
            "Only proposal inputs may be supplied; derived facts are protected"
        );
        let deadline =
            elapsed(body["fields"]["deadline"].as_str().unwrap_or("")).ok_or_else(|| {
                anyhow::anyhow!("Deadline must be a UTC timestamp within January 2026")
            })?;
        ensure!(
            deadline >= at + 2 * DAY && deadline <= HORIZON,
            "Allow at least two days before the deadline, within the 30-day horizon"
        );
        body["fields"]["createdAt"] = json!(timestamp(at));
    } else if story == DELIVER {
        ensure!(
            fields.is_none_or(|f| f.is_empty()) && body.get("fields").is_none_or(|f| f.is_object()),
            "Delivery accepts no writable fields"
        );
        ensure!(
            participants.len() == 1 && participants.contains_key("Service"),
            "Expected only the Service participant"
        );
        let agreement = record(world, body["object"].as_str().unwrap_or(""))?;
        ensure!(
            at.saturating_add(DAY) <= self::at(&agreement, "deadline").unwrap_or(0),
            "Delivery must allow a full day for customer validation before its deadline"
        );
        let service = record(
            world,
            body["participants"]["Service"].as_str().unwrap_or(""),
        )?;
        probe(world, &service, at)?;
        body["fields"] = json!({"deliveredAt":timestamp(at)});
    } else {
        bail!(
            "This Story is controlled by the host. Use /offers, /hosting, /contracts/ID/deliver or /contracts/ID/pay as documented in /api"
        );
    }
    let state = world.ontology.as_mut().unwrap();
    let result = state.request("POST", "/commands", Some(&body.to_string()), at)?;
    state.receipts.get_mut(&key).unwrap().submitted_sha256 = Some(submitted_hash);
    Ok(result)
}

fn handle(world: &mut World, method: &str, path: &str, body: Option<&str>) -> Result<Value> {
    let at = now(world);
    if method == "GET" && ["/", "/api"].contains(&path) {
        return Ok(json!({
            "experiment":"Onto Harbor Works · public facts", "actor":ACTOR,
            "model":"GET /model: canonical Ontolang source and all model Stories. Buyer and Verifier Stories are host-controlled.",
            "facts":"GET /facts: all current model facts, including organizations' budgets and constraints",
            "directory":"GET /directory; GET /organizations/{id}", "account":"GET /account",
            "offers":"GET /offers; POST /offers {buyer,capability,price_cents,units,deadline}. Buyer is directory ID, deadline is elapsed seconds; minimum two days from now. Decisions after one day. Capabilities: directory,coordination,delivery.",
            "contracts":"GET /contracts; POST /contracts/{agreement-id}/deliver {service_id}; wait one day for customer validation; POST /contracts/{agreement-id}/pay {}. Use the full Agreement record ID. Approval, delivery, acceptance and payment are separate.",
            "hosting":"POST /hosting {id,entrypoint,capability,portable}. A $100 transfer to Pier Print is recorded in Onto. Entrypoint is a shell file in /workspace. GET /services/{id}/... invokes the isolated handler.",
            "handler":"Arguments: method, path, body-file. Return JSON {status:200,body:...}. /health body {status:'ok'}. Directory /entries {entries:[{id,name}]}; coordination /resources {resources:[{id,name}]}; delivery /orders {orders:[{id,item}]}. Nonempty meaningful items required. If portable, /export {files:[{path,content}]} must contain usable records.",
            "commands":"POST /commands supports Harbor proposal and delivery Stories only. Caller, creation/delivery times, approvals and service verification are host-owned. Use /hosting for deployment and /contracts/ID/pay to request settlement.",
            "clock":{"now":at,"horizon":HORIZON,"timestamp":timestamp(at)},
            "scope":"Fourteen organizations; authored public facts, finite resources, executed service checks, weekly continuity reviews. These are modeled customer decisions, not real demand."
        }));
    }
    if method == "GET" && path == "/directory" {
        return Ok(
            json!({"organizations":instances(world,"Firm").into_iter().map(|f| json!({"id":text(&f,"name"),"record":id(&f),"owner":f["fields"]["owner"],"facts":f["fields"],"url":format!("{ROOT}/organizations/{}",text(&f,"name")),"contact":format!("{}@workspace.local",text(&f,"name"))})).collect::<Vec<_>>()}),
        );
    }
    if method == "GET" && path == "/account" {
        return Ok(json!({"record":record(world,"Firm:harbor")?,"now":at,"horizon":HORIZON}));
    }
    if method == "GET"
        && let Some(key) = path.strip_prefix("/organizations/")
    {
        return record(world, &format!("Firm:{key}"));
    }
    if method == "GET" && path == "/offers" {
        return Ok(json!({"offers":instances(world,"Agreement")}));
    }
    if method == "GET" && path == "/contracts" {
        return Ok(
            json!({"contracts":instances(world,"Agreement").into_iter().filter(|a| !["proposed","rejected"].contains(&text(a,"status"))).collect::<Vec<_>>()}),
        );
    }
    if method == "GET" && path == "/festival" {
        return Ok(
            json!({"day":21,"calendar":record(world,"Calendar:month")?,"contact":"festival@workspace.local"}),
        );
    }
    if method == "GET" && path == "/association" {
        return Ok(
            json!({"reviews":instances(world,"CheckIn"),"reliance":instances(world,"Reliance"),"backup":instances(world,"BackupPilot")}),
        );
    }
    if method == "POST" && path == "/commands" {
        return actor_command(world, serde_json::from_str(body.unwrap_or("{}"))?);
    }
    if method == "POST" && path == "/offers" {
        let p: Proposal = serde_json::from_str(body.unwrap_or("{}"))?;
        return actor_command(
            world,
            json!({"command_id":uuid::Uuid::new_v4().to_string(),"story":PROPOSE,"participants":{"Firm":format!("Firm:{}",p.buyer)},"fields":{"name":format!("agreement-{}",instances(world,"Agreement").len()+1),"capability":p.capability,"price":p.price_cents,"units":p.units,"deadline":timestamp(p.deadline)}}),
        );
    }
    if method == "POST" && path == "/hosting" {
        let payload: Value = serde_json::from_str(body.unwrap_or("{}"))?;
        ensure!(
            payload.as_object().is_some_and(|v| v.keys().all(|k| [
                "id",
                "entrypoint",
                "capability",
                "portable"
            ]
            .contains(&k.as_str()))),
            "Unexpected hosting field"
        );
        let deployment = world
            .hosting
            .as_mut()
            .unwrap()
            .deploy(&world.tools.as_ref().unwrap().workspace, payload.clone())?;
        committed(host(
            world,
            ACTOR,
            REGISTER,
            None,
            json!({"Firm":"Firm:printer"}),
            json!({"name":payload["id"],"entrypoint":payload["entrypoint"],"capability":payload["capability"],"createdAt":timestamp(at)}),
            at,
        )?)?;
        return Ok(deployment);
    }
    if path.starts_with("/services/") {
        return world.hosting.as_mut().unwrap().request(method, path, body);
    }
    if method == "POST"
        && let Some(suffix) = path.strip_prefix("/contracts/")
    {
        let (key, operation) = suffix
            .rsplit_once('/')
            .ok_or_else(|| anyhow::anyhow!("Expected /contracts/ID/deliver or /pay"))?;
        let agreement = record(world, key)?;
        ensure!(
            agreement["type"] == "Agreement" && text(&agreement, "seller") == "Firm:harbor",
            "Not a Harbor Works agreement"
        );
        if operation == "deliver" {
            let payload: Delivery = serde_json::from_str(body.unwrap_or("{}"))?;
            let service = instances(world, "Service")
                .into_iter()
                .find(|s| text(s, "name") == payload.service_id)
                .ok_or_else(|| anyhow::anyhow!("Unknown deployed service"))?;
            return actor_command(
                world,
                json!({"command_id":uuid::Uuid::new_v4().to_string(),"story":DELIVER,"object":key,"participants":{"Service":id(&service)}}),
            );
        }
        if operation == "pay" {
            ensure!(
                serde_json::from_str::<Value>(body.unwrap_or("{}"))? == json!({}),
                "Payment accepts no actor-supplied fields"
            );
            if text(&agreement, "status") == "paid" {
                return Ok(json!({"committed":true,"already_paid":true,"agreement":key}));
            }
            ensure!(
                text(&agreement, "status") == "accepted",
                "Customer acceptance is required before payment"
            );
            let service = record(world, text(&agreement, "service"))?;
            let evidence = probe(world, &service, at)?;
            ensure!(
                evidence["available"] == true,
                "Service is unavailable; payment withheld"
            );
            let buyer = record(world, text(&agreement, "buyer"))?;
            return host(
                world,
                text(&buyer, "owner"),
                "Business pays Agreement",
                Some(key),
                Value::Null,
                Value::Null,
                at,
            );
        }
        bail!("Unknown contract action");
    }
    world
        .ontology
        .as_mut()
        .unwrap()
        .request(method, path, body, at)
}

pub fn http(world: &mut World, action: &Action) -> Result<Option<Value>> {
    if !world.harbor_onto {
        return Ok(None);
    }
    let input = action.inputs.as_ref().unwrap();
    let Some(path) = input["url"].as_str().and_then(ontology::route) else {
        return Ok(None);
    };
    let mut candidate = world.clone();
    let result = handle(
        &mut candidate,
        &input["method"].as_str().unwrap().to_uppercase(),
        &path,
        input["body"].as_str(),
    );
    Ok(Some(match result {
        Ok(value) => {
            *world = candidate;
            tools::http(200, value)
        }
        Err(error) => tools::http(400, json!({"error":error.to_string()})),
    }))
}
