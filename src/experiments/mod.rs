//! Experiment assumptions live here, outside the construction engine.
pub mod bellwether;
pub mod flowers;
pub mod recall;
pub mod records;
use crate::{
    Action, Run, World,
    construction::{Directive, Interface, Request, Resolution, Resource},
    tools,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Experiment {
    Flowers,
    Records,
    Bellwether,
    Recall,
    Institutions,
}

pub fn measure(run: &Run, events: &[crate::Event]) -> Value {
    match run.experiment {
        Some(Experiment::Recall) => recall::measure(run, events),
        _ => {
            json!({"kind":"unscored","note":"No general actor-success requirement. Apply the experiment's authored measurements."})
        }
    }
}

pub fn resolve(run: &Run, action: &Action) -> Result<Resolution> {
    if ["file.read", "file.write"].contains(&action.kind.as_str())
        && action
            .inputs
            .as_ref()
            .and_then(|v| v["path"].as_str())
            .is_some_and(|p| crate::workspace::path(p).is_err())
    {
        return Ok(Resolution::Denied);
    }
    if action.kind != "http.request" {
        return Ok(
            if run
                .world
                .tools
                .as_ref()
                .map(|state| state.definitions.clone())
                .unwrap_or_else(tools::definitions)
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["name"] == action.kind)
            {
                Resolution::Existing
            } else {
                Resolution::Unsupported
            },
        );
    }
    tools::validate_action(action)?;
    let input = action.inputs.as_ref().unwrap();
    let url = reqwest::Url::parse(input["url"].as_str().unwrap())?;
    let target = url.as_str();
    if exists(&run.world, target) {
        return Ok(Resolution::Existing);
    }
    if !input["method"]
        .as_str()
        .unwrap()
        .eq_ignore_ascii_case("GET")
    {
        return Ok(Resolution::Unsupported);
    }
    let directive = match run.experiment.as_ref() {
        Some(Experiment::Flowers) => flowers::directive(target),
        Some(Experiment::Records) => records::directive(target),
        Some(Experiment::Bellwether) => bellwether::directive(target),
        Some(Experiment::Recall) => None,
        Some(Experiment::Institutions) => crate::institutions::directive(run, target),
        None => None,
    };
    Ok(match directive {
        Some(directive)
            if directive.requirements["dependencies"]
                .as_array()
                .unwrap()
                .iter()
                .all(|d| exists(&run.world, d.as_str().unwrap())) =>
        {
            Resolution::Missing {
                interface: Interface {
                    tool: "http.request".into(),
                    method: "GET".into(),
                    url: target.into(),
                },
                directive,
            }
        }
        _ => Resolution::Unsupported,
    })
}
pub fn exists(world: &World, target: &str) -> bool {
    if let Some(state) = &world.ontology
        && let Some(path) = crate::ontology::route(target)
    {
        return crate::institutions::exists(state, &path);
    }
    if world.business.is_some() && bellwether::route(target).is_some_and(|p| !p.ends_with("/about"))
    {
        return true;
    }
    if world
        .tools
        .as_ref()
        .is_some_and(|s| s.public_resources.contains_key(target))
    {
        return true;
    }
    flowers::exists(world, target)
}
pub fn context(run: &Run, target: &str) -> Result<Value> {
    match run.experiment {
        Some(Experiment::Bellwether) => bellwether::context(&run.world, target),
        Some(Experiment::Flowers) => Ok(json!({"florist":run.world.flowers})),
        Some(Experiment::Records) => {
            let directive =
                records::directive(target).ok_or_else(|| anyhow::anyhow!("Unknown boundary"))?;
            let state = run.world.tools.as_ref().unwrap();
            let mut committed = serde_json::Map::new();
            for dependency in directive.requirements["dependencies"].as_array().unwrap() {
                let key = dependency.as_str().unwrap();
                committed.insert(
                    key.into(),
                    json!(
                        state
                            .public_resources
                            .get(key)
                            .ok_or_else(|| anyhow::anyhow!("Missing dependency"))?
                    ),
                );
            }
            Ok(json!(committed))
        }
        Some(Experiment::Institutions) => crate::institutions::context(run),
        None | Some(Experiment::Recall) => {
            anyhow::bail!("No experiment construction policy")
        }
    }
}
pub fn publish(world: &mut World, request: &Request, resource: &Resource) -> Result<()> {
    match resource.functionality.as_str() {
        "onto.module.v1" => crate::institutions::publish(world, request, resource),
        "engine.bellwether-page.v1" => bellwether::publish(world, resource),
        "engine.static-http.v1" => {
            ensure!(
                request.interface.tool == "http.request" && request.interface.method == "GET",
                "Unsupported interface"
            );
            records::validate(&resource.target, &resource.initial_state, world)?;
            let state = world.tools.as_mut().unwrap();
            state.public_resources.insert(
                resource.target.clone(),
                tools::Resource {
                    title: resource.initial_state["title"].as_str().unwrap().into(),
                    snippet: "Published public record".into(),
                    status: 200,
                    headers: std::collections::BTreeMap::from([(
                        "content-type".into(),
                        "application/json".into(),
                    )]),
                    body: resource.initial_state.to_string(),
                },
            );
            Ok(())
        }
        "engine.florist-data.v1" => flowers::publish(world, resource),
        _ => anyhow::bail!("Unsupported construction type"),
    }
}
pub fn directive(target: &str, instruction: &str, requirements: Value) -> Directive {
    Directive {
        operation: "create".into(),
        target: target.into(),
        instruction: instruction.into(),
        requirements,
    }
}
