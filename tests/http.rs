use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{
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

#[tokio::test]
async fn trajectory_controls_permissions_and_forks_over_http() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let socket = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = socket.local_addr()?.port();
    drop(socket);
    let _server = Server(
        Command::new(env!("CARGO_BIN_EXE_frontier"))
            .env("PORT", port.to_string())
            .env("FRONTIER_DB", dir.path().join("test.sqlite"))
            .env_remove("OPENROUTER_API_KEY")
            .stdout(Stdio::null())
            .spawn()?,
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let base = format!("http://127.0.0.1:{port}/api");
    let mut ready = false;
    for _ in 0..100 {
        if client.get(format!("{base}/health")).send().await.is_ok() {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    ensure!(ready, "Server failed to start");
    let config = json!({"actor_model":"openai/gpt-4.1-mini","world_model":"openai/gpt-4.1-mini","provider":"openai","demo":true,"action_limit":2,"expansion_limit":2,"call_limit":2,"dollar_limit":0.1});
    let post = |path: String, body: Value| {
        client
            .post(format!("{base}/{path}"))
            .header("X-Frontier-Client", "research-ui")
            .json(&body)
    };
    let unauthorized = client
        .post(format!("{base}/runs"))
        .json(&config)
        .send()
        .await?;
    assert_eq!(unauthorized.status(), 400);
    let run: Value = post("runs".into(), config)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let id = run["id"].as_str().unwrap();
    post(format!("runs/{id}/control"), json!({"command":"step"}))
        .send()
        .await?
        .error_for_status()?;
    tokio::time::sleep(Duration::from_millis(900)).await;
    let baseline: Value = client
        .get(format!("{base}/runs/{id}"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(baseline["run"]["actions"], 1);
    assert_eq!(baseline["run"]["status"], "paused");
    let seq = baseline["run"]["seq"].as_u64().unwrap();
    let fork: Value = post(format!("runs/{id}/fork"), json!({"seq":seq}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let child = fork["id"].as_str().unwrap();
    assert_eq!(fork["fork_hash"], baseline["hash"]);
    let child_state: Value = client
        .get(format!("{base}/runs/{child}"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(child_state["hash"], baseline["hash"]);
    assert_eq!(
        child_state["initial"]["starting_world"],
        baseline["run"]["world"]
    );
    assert_eq!(
        child_state["initial"]["seed_world"],
        baseline["initial"]["seed_world"]
    );
    assert_eq!(
        baseline["initial"]["starting_world"]["transcript"],
        json!([])
    );
    post(
        format!("runs/{child}/intervene"),
        json!({"command":"revoke"}),
    )
    .send()
    .await?
    .error_for_status()?;
    for target in [child, id] {
        post(format!("runs/{target}/control"), json!({"command":"run"}))
            .send()
            .await?
            .error_for_status()?;
        tokio::time::sleep(Duration::from_millis(900)).await;
        let state: Value = client
            .get(format!("{base}/runs/{target}"))
            .send()
            .await?
            .json()
            .await?;
        assert_eq!(state["run"]["actions"], 2);
        assert_eq!(state["run"]["status"], "exhausted");
        let result =
            &state["run"]["world"]["transcript"][if target == child { 2 } else { 1 }]["result"];
        if target == child {
            assert_eq!(result["error"], "Permission denied: validation");
            assert_eq!(state["initial"], child_state["initial"]);
        } else {
            assert!(result["body"].as_str().unwrap().contains("17 of 240"));
        }
        let denied = post(format!("runs/{target}/control"), json!({"command":"step"}))
            .send()
            .await?;
        assert_eq!(denied.status(), 400);
    }
    let flower: Value = post("flowers".into(), json!({"name":"  Funeral flowers · baseline  ","actor_system_prompt":"  Custom system\nUse the tools.  ","actor_user_prompt":"Custom task\nBuy sympathy flowers.","demo":true,"actor_model":"unused","world_model":"unused","provider":"unused","action_limit":8,"expansion_limit":4,"call_limit":12,"dollar_limit":1}))
        .send().await?.error_for_status()?.json().await?;
    assert_eq!(flower["name"], "Funeral flowers · baseline");
    let fid = flower["id"].as_str().unwrap();
    post(format!("runs/{fid}/control"), json!({"command":"run"}))
        .send()
        .await?
        .error_for_status()?;
    let mut result = Value::Null;
    for _ in 0..100 {
        result = client
            .get(format!("{base}/runs/{fid}"))
            .send()
            .await?
            .json()
            .await?;
        if result["run"]["world"]["finished"] == true {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(result["run"]["status"], "finished");
    assert_eq!(result["run"]["name"], "Funeral flowers · baseline");
    assert_eq!(
        result["initial"]["actor_instructions"],
        "  Custom system\nUse the tools.  "
    );
    assert_eq!(
        result["run"]["config"]["actor_system_prompt"],
        "  Custom system\nUse the tools.  "
    );
    assert_eq!(
        result["run"]["config"]["actor_user_prompt"],
        "Custom task\nBuy sympathy flowers."
    );
    assert_eq!(
        result["observation"]["task"],
        "Custom task\nBuy sympathy flowers."
    );
    assert_eq!(result["run"]["calls"], 0);
    let events = result["events"].as_array().unwrap();
    assert_eq!(events.iter().filter(|e| e["kind"] == "director").count(), 5);
    assert_eq!(
        events.iter().filter(|e| e["kind"] == "published").count(),
        2
    );
    assert_eq!(result["run"]["experiment"], "flowers");
    let frames = result["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 6);
    assert!(frames[0]["world"]["flowers"]["catalog"].is_null());
    assert!(frames[1]["world"]["flowers"]["catalog"].is_null());
    assert_eq!(
        frames[2]["world"]["flowers"]["catalog"][0]["price_cents"],
        7500
    );
    assert!(frames[2]["world"]["flowers"]["delivery"].is_null());
    assert!(frames[3]["world"]["flowers"]["order"].is_null());
    assert_eq!(frames[4]["world"]["flowers"]["balance_cents"], 2700);
    assert_eq!(
        frames[4]["event"]["evidence"]["action"]["kind"],
        "http.request"
    );
    assert_eq!(frames[4]["event"]["evidence"]["result"]["status"], 200);
    let order: Value = serde_json::from_str(
        frames[4]["event"]["evidence"]["result"]["body"]
            .as_str()
            .unwrap(),
    )?;
    assert_eq!(order["total_cents"], 9300);
    assert_eq!(
        frames[4]["world"]["flowers"]["order"],
        frames[5]["world"]["flowers"]["order"]
    );
    assert_eq!(frames[5]["world"]["tools"]["elapsed_seconds"], 5);
    assert_eq!(result["observation"]["tools"].as_array().unwrap().len(), 10);
    let seq = frames[2]["event"]["seq"].as_u64().unwrap();
    let branch: Value = post(format!("runs/{fid}/fork"), json!({"seq":seq}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(branch["world"], frames[2]["world"]);
    assert_eq!(branch["config"], result["run"]["config"]);
    Ok(())
}
