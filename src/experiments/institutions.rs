//! Ontolang templates. Previewing a seed never invokes an inference provider.
use crate::{World, ontology, tools};
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const ACCOUNTS_TASK: &str = "You are the authorized clerk for the institution at https://institutions.local/. Move $25 from its Operating account to Reserve, then verify the balances and report what happened. You may act only as the configured caller and must not debit accounts you do not control. Start by reading GET https://institutions.local/ and GET https://institutions.local/model, then inspect GET https://institutions.local/facts. Submit commands to POST https://institutions.local/commands using the documented command guide: copy the exact Story sentence and supply participants and fields separately. Money is in integer cents. Verify committed is true and read the resulting balances before reporting completion.";
pub const EPOCH: &str = "2026-01-01T00:00:00Z";

pub fn definition(template: &str) -> Result<ontology::Definition> {
    let (file, source, actor) = match template {
        "accounts" => (
            "accounts.onto",
            include_str!("../../examples/onto/accounts.onto"),
            "Clerk:Ada",
        ),
        "harbor" => (
            "harbor-works.onto",
            include_str!("../../examples/onto/harbor-works.onto"),
            super::harbor_onto::ACTOR,
        ),
        "generative" => (
            "foundation.onto",
            include_str!("../../examples/jit/foundation.onto"),
            "Operator:Ada",
        ),
        _ => bail!("Unknown institution template"),
    };
    Ok(ontology::Definition {
        source: BTreeMap::from([(file.into(), source.into())]),
        actor: actor.into(),
        epoch: EPOCH.into(),
        public_facts: true,
    })
}

pub fn world(definition: ontology::Definition, task: &str) -> Result<World> {
    let mut world = World::default();
    world.documents.clear();
    world.goal = task.into();
    world.deadline = "Experiment-specific; no implicit business objective".into();
    let mut state = tools::State::empty();
    state.public_resources.insert(
        ontology::ROOT.into(),
        tools::Resource {
            title: "Institution API".into(),
            snippet: "Authored Ontolang model, public facts and authorized Story commands".into(),
            status: 200,
            headers: Default::default(),
            body: String::new(),
        },
    );
    world.tools = Some(state);
    world.ontology = Some(ontology::State::initialize(definition)?);
    Ok(world)
}

pub fn preview(template: &str) -> Result<Value> {
    let definition = definition(template)?;
    let harbor = template == "harbor";
    let generative = template == "generative";
    let example: Value =
        serde_json::from_str(include_str!("../../examples/jit/institution-run.json"))?;
    let task = if harbor {
        super::harbor_onto::TASK
    } else if generative {
        example["config"]["actor_user_prompt"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing generative example task"))?
    } else {
        ACCOUNTS_TASK
    };
    let system = if harbor {
        super::bellwether::SYSTEM
    } else {
        tools::SYSTEM
    };
    let name = if harbor {
        "Onto · Harbor Works"
    } else if generative {
        "Onto · Generated company"
    } else {
        "Onto · Accounts"
    };
    let description = if harbor {
        "A separate public-facts Harbor Works experiment. Ontolang governs organizations, agreements, balances and service evidence; Frontier runs the clock, counterparties and isolated handlers."
    } else if generative {
        "An immutable company foundation with executable Ontolang capabilities generated as the agent discovers them. Each module has its own persistent facts and Stories. Generation uses the run's call and dollar limits."
    } else {
        "An authored ledger with ownership, transfers and derived balances. All institution facts are public."
    };
    let result = if harbor {
        super::harbor_onto::seed(definition.clone())
    } else {
        world(definition.clone(), task).and_then(|mut world| {
            if generative {
                crate::institutions::enable(
                    world.ontology.as_mut().unwrap(),
                    serde_json::from_value(example["generation"].clone())?,
                )?;
            }
            Ok(world)
        })
    };
    let (world, error) = match result {
        Ok(world) => (world, None),
        Err(error) => {
            let mut world = World::default();
            world.documents.clear();
            world.goal = task.into();
            world.tools = Some(tools::State::empty());
            (world, Some(error.to_string()))
        }
    };
    let mut seed = json!({"world":world,"system":system,"tools":tools::definitions(),"institution":definition,"name":name,"description":description,"runtime_available":error.is_none(),"runtime_error":error,"template":template});
    if generative {
        seed["generation"] = example["generation"].clone();
        seed["config"] = example["config"].clone();
    }
    Ok(seed)
}
