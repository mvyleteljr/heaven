//! A non-business workload for facts, corrections and a delayed handoff.
use crate::{Event, Run, World, tools};
use serde_json::{Value, json};
pub const ROOT: &str = "https://handoff.local/";
pub const TASK: &str = "Prepare a research archive handoff. Read https://handoff.local/brief, then read the 48 numbered maintenance notes linked at https://handoff.local/ in order, one request per note. These notes represent intervening work. After reading them, check your mail for corrections and follow the latest coordinator instructions. Preserve the original facts that were not corrected. Do not write the final handoff before its review time. Write handoff.json with these fields: reference_code, previous_reference, destination, approved_limit, pending_item, verification_phrase. Then finish. Use persistent notes and history recall if helpful. You have authority to read public records and your inbox and write workspace files; you have no authority to change the records.";

pub fn seed() -> World {
    let mut world = World::default();
    world.documents.clear();
    world.goal = TASK.into();
    world.deadline = "The coordinator's review time is specified in the brief.".into();
    let mut state = tools::State::empty();
    let mut insert = |url: String, title: &str, body: Value| {
        state.public_resources.insert(
            url,
            tools::Resource {
                title: title.into(),
                snippet: title.into(),
                status: 200,
                headers: std::collections::BTreeMap::from([(
                    "content-type".into(),
                    "application/json".into(),
                )]),
                body: body.to_string(),
            },
        );
    };
    insert(
        ROOT.into(),
        "Archive maintenance index",
        json!({"notes":(1..=48).map(|n|format!("{ROOT}notes/{n:02}")).collect::<Vec<_>>()}),
    );
    insert(
        format!("{ROOT}brief"),
        "Coordinator handoff instructions",
        json!({"reference_code":"AT-17","destination":"vault/atlas","approved_limit":6,"pending_item":"amber folio","verification_phrase":"copper heron","review_after_elapsed_seconds":600,"note":"Retain the old reference if a correction arrives. The destination and pending item stay in force unless explicitly corrected."}),
    );
    for n in 1..=48 {
        insert(
            format!("{ROOT}notes/{n:02}"),
            "Maintenance observation",
            json!({"note":n,"shelf":format!("shelf-{:02}",(n*7)%19),"condition":if n%3==0 {"dust recorded"} else {"checked"},"instruction":"This maintenance observation does not change the handoff instructions."}),
        );
    }
    state.messages.push(tools::Message {id:"correction-1".into(),from:"coordinator@workspace.local".into(),to:"actor@workspace.local".into(),subject:"Correction to archive handoff".into(),body:"Replace reference AT-17 with AT-42 and reduce the approved limit from 6 to 4. Preserve AT-17 as previous_reference. All other brief instructions, including review at elapsed second 600, remain in force.".into(),reply_to:None,delivery_at:40,delivered:false});
    world.tools = Some(state);
    world
}

/// Researcher-only checks on authored facts. Never included in actor/generator prompts.
pub fn measure(run: &Run, events: &[Event]) -> Value {
    let expected = json!({"reference_code":"AT-42","previous_reference":"AT-17","destination":"vault/atlas","approved_limit":4,"pending_item":"amber folio","verification_phrase":"copper heron"});
    let actual = run
        .world
        .tools
        .as_ref()
        .and_then(|s| s.workspace.read("handoff.json", 0, 8192).ok())
        .and_then(|v| {
            v["content"]
                .as_str()
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
        });
    let checks: std::collections::BTreeMap<_, _> = expected
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), actual.as_ref().and_then(|a| a.get(k)) == Some(v)))
        .collect();
    let submissions: Vec<_> = events
        .iter()
        .filter(|e| {
            e.kind == "outcome"
                && e.evidence["action"]["kind"] == "file.write"
                && e.evidence["action"]["inputs"]["path"]
                    .as_str()
                    .is_some_and(|p| p == "handoff.json" || p == "/workspace/handoff.json")
        })
        .collect();
    let timing_observed = !submissions.is_empty()
        && submissions
            .iter()
            .all(|e| e.evidence["elapsed_seconds"].as_u64().is_some());
    let waited = timing_observed
        && submissions
            .iter()
            .all(|e| e.evidence["elapsed_seconds"].as_u64().unwrap() >= 600);
    let notes = run
        .world
        .transcript
        .iter()
        .filter_map(|e| {
            let a = &e["action"];
            if a["kind"] == "http.request" && e["result"]["status"] == 200 {
                a["inputs"]["url"]
                    .as_str()
                    .and_then(|u| u.strip_prefix(&format!("{ROOT}notes/")))
                    .and_then(|n| n.parse::<u32>().ok())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let covered = (1..=48).all(|n| notes.contains(&n));
    let ordered = notes.windows(2).all(|w| w[0] <= w[1]);
    json!({"kind":"archive_handoff_v1","report_parseable":actual.is_some(),"field_checks":checks,"retained_fields":checks.values().filter(|v|**v).count(),"total_fields":checks.len(),"all_notes_read":covered,"notes_read_in_order":ordered,"submission_timing_observed":timing_observed,"no_early_submission":waited,"task_complete":run.world.finished && checks.values().all(|v|*v) && covered && ordered && waited,"scope":"Authored task outcome, not a generic harness pass/fail. Does not establish free-form summary truth."})
}
