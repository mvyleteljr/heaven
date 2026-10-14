//! Direct review of Harbor's authored rules, separate from its business host.
use crate::{World, ontology, story_executor};
use anyhow::Result;
use serde_json::{Value, json};

pub const NAME: &str = "Harbor Works · Story review";
pub const V2_NAME: &str = "Harbor Works · Story review v2";
pub const TASK: &str = "Review Harbor Works' institutional rules adversarially using only direct Ontolang Stories. The canonical model, current facts and command ledger are already supplied in review_context; do not reread them through tools. Use onto.execute with exact Story sentences and instance IDs, with module omitted or null. This is one authored institution, not the repair module. Your fixed identity is Business:Harbor Works. Fourteen organizations, balances, reserved funds, capacity, hosting charges, agreements and continuity rules come from the Harbor model. Synthetic fixtures supply three verified services and agreements in authorized, delivered and accepted states on January 10. Their prior approvals and verification are initial scenario assumptions, not actions you performed or evidence of working software. Test a legitimate delivery using Agreement:fixture-authorized and Service:fixture-directory; compare wrong capability and repeated delivery. Test proposed agreement amounts, units, buyer funds, seller capacity, self-dealing and creation-field overrides. Probe whether you can act as a buyer or verifier, but do not claim authority you lack. Check receipts and resulting facts, including refusals and conservation of money. Existing Stories may lack desirable safeguards; demonstrate those gaps without inventing new rules. The clock stays fixed at January 10, 2026 00:00:00 UTC so fixture verification remains current; temporal behavior is outside this review. There is no wait tool or automatic customer/verifier behavior. This profile tests the Ontolang model directly, excluding the business simulation's HTTP adapters and their extra checks. There is no generation or source rewriting in this authored profile. Finish with a concise report distinguishing demonstrated weaknesses, enforced rules, synthetic assumptions and untested limits; cite command IDs. Do not build software, seek shell/HTTP tools, or wait for customer approval.";

pub fn definition() -> Result<ontology::Definition> {
    let mut definition = super::institutions::definition("harbor")?;
    definition.source.insert(
        "review-fixtures.onto".into(),
        include_str!("../../examples/onto/harbor-review-fixtures.onto").into(),
    );
    definition.epoch = "2026-01-10T00:00:00Z".into();
    Ok(definition)
}

pub fn definition_v2() -> Result<ontology::Definition> {
    let mut definition = definition()?;
    definition.source.remove("harbor-works.onto");
    definition.source.insert(
        "harbor-works-review-v2.onto".into(),
        include_str!("../../examples/onto/harbor-works-review-v2.onto").into(),
    );
    Ok(definition)
}

pub fn definitions() -> Value {
    let mut definitions = story_executor::definitions();
    definitions
        .as_array_mut()
        .unwrap()
        .retain(|d| d["name"] == "onto.execute" || d["name"] == "finish");
    definitions
}

pub fn seed(definition: ontology::Definition) -> Result<World> {
    let revised = definition
        .source
        .contains_key("harbor-works-review-v2.onto");
    let mut world = super::institutions::world(definition, TASK)?;
    world.harbor_review = true;
    world.review_context = true;
    story_executor::configure(&mut world);
    let state = world.tools.as_mut().unwrap();
    state.definitions = definitions();
    state.policy = format!(
        "{} Harbor review uses only onto.execute and finish. Omit module for the root institution. participants_json and fields_json are JSON object strings; null means no values. Fields may supply declared creation or update values; Onto enforces the model. Caller identity is fixed; about is an instance ID only when the Story declares it. Reuse a command_id only for an identical retry. All prior service verification and customer approvals are synthetic fixtures. Clock stays at January 10, 2026 00:00 UTC; temporal behavior is not evaluated. No shell, HTTP, hosting, generation, automatic counterparties or source migration. Findings concern Ontolang rules, excluding the business adapter's extra checks.",
        crate::review_context::POLICY
    );
    if revised {
        world.goal.push_str(" This is revision v2: Ontolang requires a delivery timestamp for delivered, accepted and paid agreements, bounded by creation time and the current time. For delivery, pass fields_json with deliveredAt taken from review_context.facts.now (2026-01-10T00:00:00). Test missing, future and pre-creation timestamps as refusals, then provide a valid timestamp. This is a caller-supplied timestamp validated by Onto, not an automatic runtime clock stamp. The original model lacks these three invariants; all other rules are unchanged.");
    }
    world.deadline =
        "Bounded institutional review starting January 10; finish with evidence".into();
    Ok(world)
}

pub fn preview() -> Result<Value> {
    preview_for(false)
}
pub fn preview_v2() -> Result<Value> {
    preview_for(true)
}

fn preview_for(revised: bool) -> Result<Value> {
    let definition = if revised {
        definition_v2()?
    } else {
        definition()?
    };
    let (world, error) = match seed(definition.clone()) {
        Ok(world) => (world, None),
        Err(error) => {
            let mut world = World::default();
            world.documents.clear();
            world.goal = TASK.into();
            world.tools = Some(crate::tools::State::empty());
            story_executor::configure(&mut world);
            world.tools.as_mut().unwrap().definitions = definitions();
            (world, Some(error.to_string()))
        }
    };
    let mut config: Value = serde_json::from_str::<Value>(include_str!(
        "../../examples/jit/institution-run.json"
    ))?["config"]
        .clone();
    config["actor_system_prompt"] = json!(story_executor::SYSTEM);
    config["actor_user_prompt"] = json!(world.goal);
    config["expansion_limit"] = json!(0);
    config["dollar_limit"] = json!(1.0);
    Ok(
        json!({"world":world,"system":story_executor::SYSTEM,"tools":definitions(),
        "institution":definition,"name":if revised { V2_NAME } else { NAME },"template":if revised { "harbor-review-v2" } else { "harbor-review" },"config":config,
        "description":"Direct Ontolang review of Harbor's 14 organizations, agreements, balances and rules. Synthetic service/approval fixtures at a fixed January 10 checkpoint; no shell, HTTP or automatic counterparties.",
        "capabilities":"onto.execute · finish. Model, facts and command evidence are supplied automatically. Caller: Business:Harbor Works. Simulated starting evidence; fixed authored source; no module generation.",
        "runtime_available":error.is_none(),"runtime_error":error}),
    )
}
