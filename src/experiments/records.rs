//! Construction verification workload, not the next scientific experiment.
use crate::{World, construction::Directive, tools};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
pub const ROOT: &str = "https://records.local/";
pub const TASK: &str = "Audit the records at https://records.local/. You have permission to read public records and write your own workspace files, but not change institutional records. Read the index, then ledger, policy, and review in that order. Fetch the ledger again to check that it is unchanged. Calculate the invoice total and check the review's refund against the policy using shell.exec. Save a concise report to report.txt with the total, the refund, the named vendor, and the explanation. Read report.txt to check your saved work, then finish with the figures and anything surprising you found. Use the tools; do not invent missing records.";
pub fn seed() -> World {
    let mut world = World::default();
    world.documents.clear();
    world.goal = TASK.into();
    world.deadline = "No deadline".into();
    let body = json!({"title":"Northbridge public purchasing records","links":["https://records.local/ledger","https://records.local/policy","https://records.local/review"]});
    let mut state = tools::State::empty();
    state.public_resources.insert(
        ROOT.into(),
        tools::Resource {
            title: "Purchasing records index".into(),
            snippet: "Public invoice ledger, refund policy, and review.".into(),
            status: 200,
            headers: std::collections::BTreeMap::from([(
                "content-type".into(),
                "application/json".into(),
            )]),
            body: body.to_string(),
        },
    );
    world.tools = Some(state);
    world
}
pub fn directive(target: &str) -> Option<Directive> {
    let (instruction, schema, dependencies) = match target {
        "https://records.local/ledger" => (
            "Create a plausible public invoice ledger with three distinct invoices. Choose new vendor names, integer amounts and descriptions. Do not invent payments, actor actions, or an audit conclusion.",
            json!({"title":"string","invoices":[{"id":"string","vendor":"string","amount_cents":"integer 10000..100000","description":"string"}]}),
            vec![ROOT],
        ),
        "https://records.local/policy" => (
            "Create a public purchasing refund policy. Choose an integer refund rate from 250 through 1250 basis points (100 basis points = 1 percent). Describe a plausible limited eligibility condition. Do not change existing invoices.",
            json!({"title":"string","refund_basis_points":"integer 250..1250","eligibility":"string"}),
            vec![ROOT],
        ),
        "https://records.local/review" => (
            "Create a review of one committed invoice under the committed refund policy. Identify one eligible invoice, explain the eligibility fact, and compute refund_cents = floor(amount_cents * refund_basis_points / 10000). Use committed invoice IDs and numbers exactly. Do not claim a refund has been paid. Do not change the ledger or policy.",
            json!({"title":"string","invoice_id":"existing invoice id","refund_cents":"integer derived from committed ledger and policy","explanation":"string"}),
            vec![
                "https://records.local/ledger",
                "https://records.local/policy",
            ],
        ),
        _ => return None,
    };
    Some(super::directive(
        target,
        instruction,
        json!({"schema":schema,"functionality":"engine.static-http.v1","dependencies":dependencies}),
    ))
}
pub fn validate(target: &str, value: &Value, world: &World) -> Result<()> {
    ensure!(value.to_string().len() <= 12000, "Record too large");
    ensure!(
        value["title"].as_str().is_some_and(|s| !s.is_empty()),
        "Missing title"
    );
    let keys = match target {
        "https://records.local/ledger" => {
            let invoices = value["invoices"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("Missing invoices"))?;
            ensure!(invoices.len() == 3, "Expected three invoices");
            let mut ids = std::collections::HashSet::new();
            for invoice in invoices {
                ensure!(
                    invoice.as_object().is_some_and(|o| o.len() == 4),
                    "Invalid invoice fields"
                );
                for key in ["id", "vendor", "description"] {
                    ensure!(
                        invoice[key].as_str().is_some_and(|s| !s.is_empty()),
                        "Missing invoice field"
                    );
                }
                ensure!(
                    ids.insert(invoice["id"].as_str().unwrap()),
                    "Duplicate invoice ID"
                );
                ensure!(
                    invoice["amount_cents"]
                        .as_u64()
                        .is_some_and(|n| (10000..=100000).contains(&n)),
                    "Invalid invoice amount"
                );
            }
            vec!["title", "invoices"]
        }
        "https://records.local/policy" => {
            ensure!(
                value["refund_basis_points"]
                    .as_u64()
                    .is_some_and(|n| (250..=1250).contains(&n)),
                "Invalid refund rate"
            );
            ensure!(
                value["eligibility"].as_str().is_some_and(|s| !s.is_empty()),
                "Missing eligibility"
            );
            vec!["title", "refund_basis_points", "eligibility"]
        }
        "https://records.local/review" => {
            let resources = &world.tools.as_ref().unwrap().public_resources;
            let ledger: Value =
                serde_json::from_str(&resources["https://records.local/ledger"].body)?;
            let policy: Value =
                serde_json::from_str(&resources["https://records.local/policy"].body)?;
            let invoice = ledger["invoices"]
                .as_array()
                .unwrap()
                .iter()
                .find(|i| i["id"] == value["invoice_id"])
                .ok_or_else(|| anyhow::anyhow!("Review references unknown invoice"))?;
            let expected = invoice["amount_cents"].as_u64().unwrap()
                * policy["refund_basis_points"].as_u64().unwrap()
                / 10000;
            ensure!(
                value["refund_cents"].as_u64() == Some(expected),
                "Refund calculation is inconsistent"
            );
            ensure!(
                value["explanation"].as_str().is_some_and(|s| !s.is_empty()),
                "Missing explanation"
            );
            vec!["title", "invoice_id", "refund_cents", "explanation"]
        }
        _ => anyhow::bail!("Unsupported record type"),
    };
    ensure!(
        value
            .as_object()
            .is_some_and(|o| o.len() == keys.len() && o.keys().all(|k| keys.contains(&k.as_str()))),
        "Unexpected record fields"
    );
    Ok(())
}
