//! Researcher-only measurements. These never steer actors or score task success.
use crate::{Event, Run};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub fn report(run: &Run, events: &[Event]) -> Value {
    let mut actions = BTreeMap::<String, usize>::new();
    let mut fingerprints = BTreeMap::<String, usize>::new();
    let mut failures = 0;
    let mut longest_repeat = 0;
    let mut streak = 0;
    let mut previous = String::new();
    for entry in &run.world.transcript {
        let action = &entry["action"];
        *actions
            .entry(action["kind"].as_str().unwrap_or("unknown").into())
            .or_default() += 1;
        let fingerprint = format!("{:x}", Sha256::digest(action.to_string().as_bytes()));
        *fingerprints.entry(fingerprint.clone()).or_default() += 1;
        streak = if fingerprint == previous {
            streak + 1
        } else {
            1
        };
        longest_repeat = longest_repeat.max(streak);
        previous = fingerprint;
        if entry["result"].get("error").is_some()
            || entry["result"]["status"].as_u64().is_some_and(|n| n >= 400)
        {
            failures += 1;
        }
    }
    let mut roles = BTreeMap::<String, usize>::new();
    let mut event_counts = BTreeMap::<String, usize>::new();
    let mut error_events = Vec::new();
    let mut last_summary = None;
    let mut unchanged_summaries = 0;
    let mut memory_changes = Vec::new();
    for event in events {
        *event_counts.entry(event.kind.clone()).or_default() += 1;
        if event.kind == "inference" {
            *roles
                .entry(event.evidence["role"].as_str().unwrap_or("unknown").into())
                .or_default() += 1;
        }
        if [
            "error",
            "inference_failure",
            "construction_rejected",
            "actor_rejected",
        ]
        .contains(&event.kind.as_str())
        {
            error_events.push(json!({"seq":event.seq,"kind":event.kind,"role":event.evidence.get("role"),"phase":event.evidence.get("phase"),"message":event.evidence.get("error")}));
        }
        if event.kind == "memory" {
            let summary = &event.evidence["memory"]["summary"];
            let same = last_summary == Some(summary);
            if same {
                unchanged_summaries += 1;
            }
            memory_changes.push(json!({"seq":event.seq,"covered_entries":event.evidence["memory"]["covered_entries"],"items":summary["items"].as_array().map(Vec::len),"unchanged_from_previous_publication":same}));
            last_summary = Some(summary);
        }
    }
    let memory_prefix_valid = run.memory.as_ref().map(|m| {
        m.covered_entries <= run.world.transcript.len()
            && (m.covered_entries == 0
                || m.prefix_hash
                    == format!(
                        "{:x}",
                        Sha256::digest(
                            serde_json::to_vec(&run.world.transcript[..m.covered_entries]).unwrap()
                        )
                    ))
    });
    json!({
        "version":1,"run_id":run.id,"checkpoint_hash":run.checkpoint_hash(),"status":run.status,"busy":run.busy,"experiment":run.experiment,"reporting_source_sha256":env!("FRONTIER_SOURCE_SHA256"),
        "condition":{"config":run.config,"scope":"Saved configuration; runtime source revision must be recorded separately by the audit runner."},
        "integrity":{"events_contiguous":events.iter().enumerate().all(|(i,e)| e.seq == i as u64 + 1),"last_event_matches_run":events.last().is_some_and(|e| e.seq == run.seq),"completed_observations":run.world.transcript.len(),"attempted_actions":run.actions,"unresolved_action_attempts":(run.actions as usize).saturating_sub(run.world.transcript.len()),"memory_prefix_valid":memory_prefix_valid},
        "spending":{"calls_including_inherited":run.calls,"charged_usd":run.charged,"uncertain_reserved_usd":run.reserved,"calls_recorded_on_this_branch":roles,"limit_usd":run.config.dollar_limit},
        "behavior":{"action_counts":actions,"failed_tool_results":failures,"distinct_actions":fingerprints.len(),"repeated_actions_after_first":run.world.transcript.len().saturating_sub(fingerprints.len()),"longest_identical_action_streak":longest_repeat,"elapsed_seconds":run.world.tools.as_ref().map(|t|t.elapsed_seconds),"note":"Repetition and tool refusals are observations, not harness failures or automatic intervention triggers."},
        "construction":{"attempts_including_inherited":run.expansions,"published_on_this_branch":event_counts.get("published").copied().unwrap_or(0),"rejected_on_this_branch":event_counts.get("construction_rejected").copied().unwrap_or(0),"timings":events.iter().filter(|e|e.kind=="published" && e.evidence.get("latency_ms").is_some()).map(|e|json!({"seq":e.seq,"latency_ms":e.evidence["latency_ms"],"validation_ms":e.evidence["validation_ms"],"request_bytes":e.evidence["request_bytes"]})).collect::<Vec<_>>()},
        "memory":{"revisions_including_inherited":run.memory.as_ref().map_or(0,|m|m.revisions),"unchanged_publications":unchanged_summaries,"publications":memory_changes,"note":"Structural integrity and unchanged text do not measure semantic fidelity."},
        "events":event_counts,"failures":error_events,
        "outcome":crate::experiments::measure(run,events)
    })
}
