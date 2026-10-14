//! Institution setup routes never start inference. The child uses a dummy key,
//! no endpoint settings, an unreachable outbound proxy, and no Step/Run requests.
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn server(
    database: &Path,
    runtime: Option<&Path>,
) -> Result<(Server, reqwest::Client, String)> {
    let socket = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = socket.local_addr()?.port();
    drop(socket);
    let mut command = Command::new(env!("CARGO_BIN_EXE_frontier"));
    command
        .env("HOST", "127.0.0.1")
        .env("PORT", port.to_string())
        .env("FRONTIER_DB", database)
        .env("OPENROUTER_API_KEY", "dummy-never-used-by-paused-setup")
        .env("HTTPS_PROXY", "http://127.0.0.1:9")
        .env("HTTP_PROXY", "http://127.0.0.1:9")
        .env("NO_PROXY", "localhost,127.0.0.1")
        .env_remove("FRONTIER_ONTO_RUNTIME")
        .stdout(Stdio::null());
    if let Some(runtime) = runtime {
        command.env("FRONTIER_ONTO_RUNTIME", runtime);
    }
    let server = Server(command.spawn()?);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;
    let base = format!("http://127.0.0.1:{port}/api");
    for _ in 0..100 {
        if client.get(format!("{base}/health")).send().await.is_ok() {
            return Ok((server, client, base));
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    anyhow::bail!("Server failed to start")
}

fn config(task: &str) -> Value {
    json!({
        "actor_model":"openai/gpt-4.1-mini", "world_model":"openai/gpt-4.1-mini",
        "provider":"openai", "demo":false, "structured_actions":false,
        "actor_settings":null, "generator_settings":null, "memory":null,
        "actor_user_prompt":task, "action_limit":10, "expansion_limit":4,
        "call_limit":12, "dollar_limit":0.25
    })
}

fn post(client: &reqwest::Client, url: String, body: &Value) -> reqwest::RequestBuilder {
    client
        .post(url)
        .header("X-Frontier-Client", "research-ui")
        .json(body)
}

async fn read(client: &reqwest::Client, url: String) -> Result<Value> {
    Ok(client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

fn assert_paused(run: &Value) {
    assert_eq!(run["status"], "paused");
    assert_eq!(run["busy"], false);
    assert_eq!(run["calls"], 0);
    assert_eq!(run["actions"], 0);
    assert_eq!(run["charged"], 0.0);
    assert_eq!(run["reserved"], 0.0);
}

#[tokio::test]
#[ignore = "Requires the pinned optional Onto bundle"]
async fn story_review_saves_its_restricted_interface_without_starting_inference() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let runtime = std::env::var("FRONTIER_ONTO_RUNTIME")?;
    let (_server, client, base) =
        server(&dir.path().join("review.sqlite"), Some(Path::new(&runtime))).await?;
    let seed = read(
        &client,
        format!("{base}/institution-seed?template=story-review"),
    )
    .await?;
    assert_eq!(seed["runtime_available"], true);
    let run: Value = post(&client, format!("{base}/institutions"), &json!({
        "template":"story-review", "institution":seed["institution"],
        "generation":seed["generation"], "config":config(seed["world"]["goal"].as_str().unwrap())
    })).send().await?.error_for_status()?.json().await?;
    assert_paused(&run);
    assert_eq!(run["experiment"], "onto_review");
    let names: Vec<_> = run["world"]["tools"]["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        vec!["onto.inspect", "onto.execute", "onto.expand", "finish"]
    );
    assert_eq!(run["world"]["ontology"]["generation"], seed["generation"]);
    assert_eq!(seed["generation"]["repair_attempts"], 2);
    assert_eq!(
        run["world"]["ontology"]["modules"]["repair"],
        seed["world"]["ontology"]["modules"]["repair"]
    );
    assert_eq!(
        run["world"]["ontology"]["modules"]["repair"]["stories"]
            .as_array()
            .unwrap()
            .len(),
        5
    );
    assert!(run["world"]["hosting"].is_null());
    assert!(
        run["config"]["actor_system_prompt"]
            .as_str()
            .unwrap()
            .contains("institution")
    );
    Ok(())
}

#[tokio::test]
#[ignore = "Requires the pinned optional Onto bundle"]
async fn harbor_review_saves_fixed_direct_stories_and_rejects_identity_or_source_changes()
-> Result<()> {
    for (template, experiment, name) in [
        (
            "harbor-review",
            "harbor_review",
            "Harbor Works · Story review",
        ),
        (
            "harbor-review-v2",
            "harbor_review_v2",
            "Harbor Works · Story review v2",
        ),
    ] {
        let dir = tempfile::tempdir()?;
        let database = dir.path().join("harbor-review.sqlite");
        let runtime = std::env::var("FRONTIER_ONTO_RUNTIME")?;
        let (handle, client, base) = server(&database, Some(Path::new(&runtime))).await?;
        let seed = read(
            &client,
            format!("{base}/institution-seed?template={template}"),
        )
        .await?;
        assert_eq!(seed["runtime_available"], true, "{seed}");
        assert_eq!(seed["name"], name);
        assert_eq!(
            seed["world"]["goal"]
                .as_str()
                .unwrap()
                .contains("This is revision v2"),
            template == "harbor-review-v2"
        );
        assert_eq!(seed["config"]["actor_user_prompt"], seed["world"]["goal"]);
        assert!(seed["generation"].is_null());
        assert_eq!(seed["config"]["expansion_limit"], 0);
        assert_eq!(seed["config"]["actor_model"], "openai/gpt-6.1-sol");
        assert_eq!(seed["config"]["world_model"], "openai/gpt-6.1-sol");
        let request = json!({"template":template, "institution":seed["institution"], "config":config(seed["world"]["goal"].as_str().unwrap())});
        for changed in ["actor", "epoch", "source", "agent_state", "generation"] {
            let mut invalid = request.clone();
            match changed {
                "actor" => invalid["institution"]["actor"] = json!("Business:Quay Hall"),
                "epoch" => invalid["institution"]["epoch"] = json!("2026-01-01T00:00:00Z"),
                "agent_state" => invalid["institution"]["agent_state"] = json!({"fields":[]}),
                "source" => {
                    invalid["institution"]["source"]["review-fixtures.onto"] =
                        json!("// missing fixtures")
                }
                _ => invalid["generation"] = json!({"brief":"Rewrite Harbor", "max_modules":4}),
            }
            assert_eq!(
                post(&client, format!("{base}/institutions"), &invalid)
                    .send()
                    .await?
                    .status(),
                400,
                "{changed}"
            );
        }
        assert!(
            read(&client, format!("{base}/runs"))
                .await?
                .as_array()
                .unwrap()
                .is_empty()
        );
        let run: Value = post(&client, format!("{base}/institutions"), &request)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        assert_paused(&run);
        assert_eq!(run["name"], name);
        assert_eq!(run["experiment"], experiment);
        assert_eq!(run["world"]["ontology"], seed["world"]["ontology"]);
        assert!(run["world"]["hosting"].is_null());
        assert!(run["world"]["harbor_onto"] != true);
        assert_eq!(
            run["world"]["tools"]["definitions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|d| d["name"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["onto.execute", "finish"]
        );
        let id = run["id"].as_str().unwrap();
        let before = read(&client, format!("{base}/runs/{id}")).await?;
        drop(handle);
        let (_handle, client, base) = server(&database, Some(Path::new(&runtime))).await?;
        let after = read(&client, format!("{base}/runs/{id}")).await?;
        assert_eq!(after["run"], before["run"]);
        assert_eq!(after["hash"], before["hash"]);
        assert_paused(&after["run"]);
    }
    Ok(())
}

#[tokio::test]
#[ignore = "Requires the pinned optional Onto bundle and the Linux sandbox"]
async fn institution_templates_validate_and_create_independent_paused_runs() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let database = dir.path().join("institutions.sqlite");
    let runtime = std::env::var("FRONTIER_ONTO_RUNTIME")?;
    ensure!(
        Path::new(&runtime).is_file(),
        "Build the optional Onto runtime first"
    );
    let (server_handle, client, base) = server(&database, Some(Path::new(&runtime))).await?;

    let account_seed = read(
        &client,
        format!("{base}/institution-seed?template=accounts"),
    )
    .await?;
    let harbor_seed = read(&client, format!("{base}/institution-seed?template=harbor")).await?;
    for (template, seed) in [("accounts", &account_seed), ("harbor", &harbor_seed)] {
        assert_eq!(seed["template"], template);
        assert_eq!(seed["runtime_available"], true, "{seed}");
        assert!(seed["runtime_error"].is_null());
        assert_eq!(seed["institution"]["public_facts"], true);
        assert!(
            !seed["institution"]["source"]
                .as_object()
                .unwrap()
                .is_empty()
        );
        assert!(seed["world"]["ontology"].is_object());
        assert!(seed["world"]["business"].is_null());
    }
    assert_eq!(account_seed["institution"]["actor"], "Clerk:Ada");
    assert_eq!(harbor_seed["institution"]["actor"], "Business:Harbor Works");
    assert_eq!(harbor_seed["world"]["harbor_onto"], true);
    assert!(account_seed["world"]["harbor_onto"] != true);
    let count = harbor_seed["world"]["ontology"]["snapshot"]["instances"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|record| record["type"] == "Firm")
        .count();
    assert_eq!(count, 14);

    // Preserve an original Harbor Works run beside both new institution runs.
    let mut original_body = config("Keep the original Harbor Works experiment paused.");
    original_body["name"] = json!("Original Harbor Works");
    let original: Value = post(&client, format!("{base}/bellwether"), &original_body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_paused(&original);
    assert_eq!(original["experiment"], "bellwether");
    assert!(original["world"]["business"].is_object());
    assert!(original["world"]["ontology"].is_null());
    let original_id = original["id"].as_str().unwrap().to_owned();

    let mut account_definition = account_seed["institution"].clone();
    let account_source = account_definition["source"]["accounts.onto"]
        .as_str()
        .unwrap();
    account_definition["source"]["accounts.onto"] =
        json!(account_source.replace("opening: 100.00", "opening: 125.00"));
    let account_request = json!({
        "template":"accounts", "name":"Edited Onto Accounts", "institution":account_definition,
        "config":config("Inspect this edited model without executing actions.")
    });
    let unauthorized = client
        .post(format!("{base}/institutions"))
        .json(&account_request)
        .send()
        .await?;
    assert_eq!(unauthorized.status(), 400);
    let cross_site = post(&client, format!("{base}/institutions"), &account_request)
        .header("Sec-Fetch-Site", "cross-site")
        .send()
        .await?;
    assert_eq!(cross_site.status(), 400);

    let mut invalid = account_request.clone();
    invalid["institution"]["source"]["accounts.onto"] = json!("this is not an Ontolang model");
    let response = post(&client, format!("{base}/institutions"), &invalid)
        .send()
        .await?;
    assert_eq!(response.status(), 400);
    let error: Value = response.json().await?;
    assert!(error["error"].as_str().unwrap().contains("Onto refused"));
    invalid = account_request.clone();
    invalid["institution"]["public_facts"] = json!(false);
    let response = post(&client, format!("{base}/institutions"), &invalid)
        .send()
        .await?;
    assert_eq!(response.status(), 400);

    let harbor_request = json!({
        "template":"harbor", "name":"Separate Onto Harbor Works", "institution":harbor_seed["institution"],
        "config":config("Inspect the Ontolang Harbor Works world without executing actions.")
    });
    for field in ["actor", "epoch", "source"] {
        let mut forged = harbor_request.clone();
        forged["institution"][field] = match field {
            "actor" => json!("Business:Quay Hall"),
            "epoch" => json!("2026-02-01T00:00:00Z"),
            _ => account_seed["institution"]["source"].clone(),
        };
        let response = post(&client, format!("{base}/institutions"), &forged)
            .send()
            .await?;
        assert_eq!(response.status(), 400);
        let error: Value = response.json().await?;
        assert!(
            error["error"]
                .as_str()
                .unwrap()
                .contains("authored model, caller and epoch")
        );
    }

    let mut saved = vec![];
    for (request, experiment) in [
        (&account_request, "institutions"),
        (&harbor_request, "harbor_onto"),
    ] {
        let run: Value = post(&client, format!("{base}/institutions"), request)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        assert_paused(&run);
        assert_eq!(run["experiment"], experiment);
        assert_eq!(run["name"], request["name"]);
        assert!(run["world"]["business"].is_null());
        assert_eq!(
            run["world"]["harbor_onto"] == true,
            experiment == "harbor_onto"
        );
        assert_eq!(
            run["world"]["ontology"]["definition"]["actor"],
            request["institution"]["actor"]
        );
        let id = run["id"].as_str().unwrap().to_owned();
        assert_ne!(id, original_id);
        let detail = read(&client, format!("{base}/runs/{id}")).await?;
        assert_eq!(detail["run"], run);
        assert_eq!(detail["initial"]["starting_world"], run["world"]);
        assert_eq!(detail["initial"]["seed_world"], run["world"]);
        assert_eq!(run["world"]["goal"], request["config"]["actor_user_prompt"]);
        assert_eq!(
            detail["initial"]["actor_instructions"],
            run["config"]["actor_system_prompt"]
        );
        assert!(
            run["world"]["ontology"]["runtime_sha256"]
                .as_str()
                .unwrap()
                .len()
                == 64
        );
        if experiment == "institutions" {
            let operating = run["world"]["ontology"]["snapshot"]["instances"]
                .as_array()
                .unwrap()
                .iter()
                .find(|record| record["id"] == "Account:Operating")
                .unwrap();
            assert_eq!(operating["fields"]["balance"], 12500);
        }
        saved.push((id, detail));
    }
    assert_ne!(saved[0].0, saved[1].0);
    let original_after = read(&client, format!("{base}/runs/{original_id}")).await?;
    assert_eq!(original_after["run"], original);

    // Reload the actual database in a second process, without ever running an actor.
    drop(server_handle);
    let (_restored_server, client, base) = server(&database, Some(Path::new(&runtime))).await?;
    for (id, detail) in saved {
        let restored = read(&client, format!("{base}/runs/{id}")).await?;
        assert_paused(&restored["run"]);
        assert_eq!(restored["run"], detail["run"]);
        assert_eq!(restored["initial"], detail["initial"]);
        assert_eq!(restored["hash"], detail["hash"]);
    }
    Ok(())
}

#[tokio::test]
async fn missing_runtime_preserves_template_source_and_existing_seed_routes() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let (_server, client, base) = server(&dir.path().join("missing-runtime.sqlite"), None).await?;
    for template in [
        "accounts",
        "harbor",
        "generative",
        "story-review",
        "harbor-review",
        "harbor-review-v2",
    ] {
        let seed = read(
            &client,
            format!("{base}/institution-seed?template={template}"),
        )
        .await?;
        assert_eq!(seed["runtime_available"], false);
        assert!(
            seed["runtime_error"]
                .as_str()
                .unwrap()
                .contains("not configured")
        );
        assert!(
            !seed["institution"]["source"]
                .as_object()
                .unwrap()
                .is_empty()
        );
        assert!(seed["world"]["ontology"].is_null());
    }
    let unknown = client
        .get(format!("{base}/institution-seed?template=unknown"))
        .send()
        .await?;
    assert_eq!(unknown.status(), 400);
    let flower = read(&client, format!("{base}/flower-seed")).await?;
    assert!(flower["world"]["tools"].is_object());
    assert!(flower["world"]["ontology"].is_null());
    assert!(!flower["tools"].as_array().unwrap().is_empty());
    Ok(())
}

#[tokio::test]
#[ignore = "Requires the pinned optional Onto bundle and the Linux sandbox"]
async fn generative_template_saves_a_paused_foundation_and_keeps_authored_models_separate()
-> Result<()> {
    let dir = tempfile::tempdir()?;
    let runtime = std::env::var("FRONTIER_ONTO_RUNTIME")?;
    let (_server, client, base) = server(
        &dir.path().join("generative.sqlite"),
        Some(Path::new(&runtime)),
    )
    .await?;
    let seed = read(
        &client,
        format!("{base}/institution-seed?template=generative"),
    )
    .await?;
    assert_eq!(seed["runtime_available"], true, "{seed}");
    assert_eq!(seed["template"], "generative");
    assert_eq!(seed["world"]["ontology"]["stories"], json!([]));
    assert_eq!(seed["world"]["ontology"]["generation"], seed["generation"]);
    assert_eq!(seed["config"]["structured_actions"], true);
    let mut request = json!({
        "template":"generative", "institution":seed["institution"],
        "generation":seed["generation"],
        "config":config("Discover capabilities while preserving the foundation.")
    });
    let run: Value = post(&client, format!("{base}/institutions"), &request)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_paused(&run);
    assert_eq!(run["experiment"], "institutions");
    assert_eq!(run["name"], "Onto · Generated company");
    assert_eq!(run["world"]["ontology"]["generation"], seed["generation"]);
    assert!(
        run["world"]["ontology"]["modules"]
            .as_object()
            .is_none_or(|m| m.is_empty())
    );

    request.as_object_mut().unwrap().remove("generation");
    let response = post(&client, format!("{base}/institutions"), &request)
        .send()
        .await?;
    assert_eq!(
        response.status(),
        400,
        "A generative template must enable generation"
    );

    for template in ["accounts", "harbor"] {
        let authored = read(
            &client,
            format!("{base}/institution-seed?template={template}"),
        )
        .await?;
        assert!(authored["world"]["ontology"]["generation"].is_null());
        request["template"] = json!(template);
        request["institution"] = authored["institution"].clone();
        request["generation"] = seed["generation"].clone();
        let response = post(&client, format!("{base}/institutions"), &request)
            .send()
            .await?;
        assert_eq!(
            response.status(),
            400,
            "Authored Stories cannot be a generative foundation"
        );
    }
    Ok(())
}
