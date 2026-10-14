//! Deterministic command-evidence projection for authored reviews. Saved history is untouched.
use crate::{Run, story_executor};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub const POLICY: &str = "Authored review context v1. The host supplies one canonical model, one current fact snapshot and an indexed command ledger on every turn. They are exact runtime evidence, not a model summary. Unchanged inspection snapshots are omitted from the prompt; full history remains saved for audit. Do not inspect again: execute a Story or finish. Ledger field changes preserve before/after values; missing fields are marked explicitly. A refused command tests a constraint but does not change facts. Invalid inputs and replays are not new successful executions. When review_budget.finish_only is true, finish now with a partial report, command IDs, enforced rules, demonstrated gaps and untested checks.";

fn compact_changes(changes: &Value) -> Value {
    json!(changes.as_array().into_iter().flatten().map(|change| {
        let before = change["before"]["fields"].as_object();
        let after = change["after"]["fields"].as_object();
        match (before, after) {
            (Some(before), Some(after)) => {
                let keys: BTreeSet<_> = before.keys().chain(after.keys()).collect();
                let fields: serde_json::Map<_, _> = keys.into_iter()
                    .filter(|key| before.get(*key) != after.get(*key))
                    .map(|key| (key.clone(), json!({"before":before.get(key),"after":after.get(key),
                        "before_present":before.contains_key(key),"after_present":after.contains_key(key)})))
                    .collect();
                json!({"id":change["id"],"fields":fields})
            }
            _ => change.clone(), // Creation/deletion retains the entire record.
        }
    }).collect::<Vec<_>>())
}

pub fn ledger(run: &Run) -> Vec<Value> {
    run.world
        .transcript
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            // Invalid inspections remain visible as corrective feedback.
            if entry["action"]["kind"] == "onto.inspect" && entry["result"].get("error").is_none() {
                return None;
            }
            let mut result = entry["result"].clone();
            if let Some(changes) = result.get("changes") {
                result["changes"] = compact_changes(changes);
            }
            Some(json!({"history_index":index,"action":entry["action"],"result":result}))
        })
        .collect()
}

pub fn observation(run: &Run) -> Value {
    let state = run
        .world
        .ontology
        .as_ref()
        .expect("Authored review requires Onto");
    let ledger = ledger(run);
    let attempted: BTreeSet<_> = ledger
        .iter()
        .filter_map(|e| e["action"]["inputs"]["story"].as_str())
        .collect();
    let unattempted: Vec<_> = state
        .stories
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s["story"].as_str())
        .filter(|s| !attempted.contains(s))
        .collect();
    json!({"task":run.world.goal,"tools":story_executor::available_definitions(run),
        "tool_policy":run.world.tools.as_ref().unwrap().policy,
        "review_context":{"version":1,"actor":state.definition.actor,
            "model":state.definition.source,"stories":state.stories,"facts":state.snapshot,
            "agent_state":state.agent_state,
            "command_ledger":ledger,"saved_history_entries":run.world.transcript.len(),
            "unattempted_stories":unattempted,
            "coverage_notice":"An attempted Story is not complete boundary coverage; report untested task checks explicitly."},
        "budget":{"charged_usd":run.charged,"reserved_usd":run.reserved,
            "remaining_usd":(run.config.dollar_limit-run.charged-run.reserved).max(0.0),
            "actions_remaining":run.config.action_limit.saturating_sub(run.actions),
            "calls_remaining":run.config.call_limit.saturating_sub(run.calls)}})
}

/// No inference and no fabricated actor action. Only recorded commands support this report.
pub fn partial_report(run: &Run, reason: &str) -> Value {
    let ledger = ledger(run);
    let commands: Vec<_> = ledger
        .iter()
        .filter(|e| e["action"]["kind"] == "onto.execute")
        .collect();
    let mut lines = vec![
        format!(
            "Partial review — {reason}. This is a host-generated evidence summary; the agent did not complete its review."
        ),
        format!(
            "{} Story attempts recorded; ${:.4} charged. No further inference was sent.",
            commands.len(),
            run.charged
        ),
    ];
    for entry in &commands {
        let result = &entry["result"];
        let outcome = if result["replayed"] == true {
            "receipt replay"
        } else if result["committed"] == true {
            "committed"
        } else if result["committed"] == false {
            "refused"
        } else {
            "not executed / input error"
        };
        lines.push(format!(
            "• {}: {} — {}",
            entry["action"]["inputs"]["command_id"]
                .as_str()
                .unwrap_or("unnamed"),
            entry["action"]["inputs"]["story"]
                .as_str()
                .unwrap_or("unknown Story"),
            outcome
        ));
    }
    lines.push("Unexecuted checks remain unverified. Synthetic initial facts are assumptions. Fixed-clock review does not evaluate temporal behavior or the business adapter.".into());
    json!({"source":"host","partial":true,"reason":reason,"message":lines.join("\n"),
        "command_ledger":ledger,"coverage":observation(run)["review_context"]["unattempted_stories"]})
}
