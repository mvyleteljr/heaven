use anyhow::Result;
use frontier::hosting::State;
use frontier::workspace;
use serde_json::json;

fn deploy(script: &str, capability: &str, portable: bool) -> Result<State> {
    let mut w = workspace::Workspace::default();
    w.write("handler.sh", script)?;
    let mut s = State::default();
    s.deploy(
        &w,
        json!({"id":"shop","entrypoint":"handler.sh","capability":capability,"portable":portable}),
    )?;
    Ok(s)
}

#[test]
fn executable_snapshot_persists_and_branches_without_host_access() -> Result<()> {
    let mut s = deploy(
        r#"
case "$2" in
/health) echo '{"status":200,"body":{"working":true}}';;
/save) cat "$3" > data; echo '{"status":201,"body":"saved"}';;
/read) printf '{"status":200,"body":"'; cat data; printf '"}\n';;
/isolation)
test ! -e /etc/passwd || exit 1
test ! -e /home/user || exit 1
# No host interface is present in the unshared network namespace.
test "$(ls /sys/class/net 2>/dev/null)" = '' || exit 1
/bin/busybox wget -T 1 -q -O /tmp/network http://1.1.1.1 2>/dev/null && exit 1
echo '{"status":200,"body":"isolated"}';;
esac
"#,
        "delivery",
        false,
    )?;
    let payload = "a'$(touch injected);safe";
    s.request("POST", "/services/shop/save", Some(payload))?;
    assert_eq!(
        s.request("GET", "/services/shop/read", None)?["body"],
        payload
    );
    assert_eq!(
        s.request("GET", "/services/shop/isolation", None)?["body"],
        "isolated"
    );
    let mut branch = s.clone();
    branch.request("POST", "/services/shop/save", Some("branch"))?;
    assert_eq!(
        s.request("GET", "/services/shop/read", None)?["body"],
        payload
    );
    let mut restored: State = serde_json::from_value(serde_json::to_value(&branch)?)?;
    assert_eq!(restored, branch);
    assert_eq!(
        restored.request("GET", "/services/shop/read", None)?["body"],
        "branch"
    );
    Ok(())
}

#[test]
fn probe_requires_public_payload_and_export() -> Result<()> {
    let mut declaration = deploy(
        "echo '{\"status\":200,\"body\":{\"working\":true,\"available\":true,\"portable\":true}}'",
        "directory",
        true,
    )?;
    let p = declaration.probe("shop")?;
    assert_eq!(p["available"], false);
    assert_eq!(p["portable"], false);
    for (cap, endpoint, key, label) in [
        ("directory", "entries", "entries", "name"),
        ("coordination", "resources", "resources", "name"),
        ("delivery", "orders", "orders", "item"),
    ] {
        let script = format!(
            r#"
case "$2" in
/health) echo '{{"status":200,"body":{{"status":"ok"}}}}';;
/{endpoint}) echo '{{"status":200,"body":{{"{key}":[{{"id":"one","{label}":"Fresh food"}}]}}}}';;
/export) echo '{{"status":200,"body":{{"files":[{{"path":"data.json","content":"Fresh food"}}]}}}}';;
esac
"#
        );
        let mut s = deploy(&script, cap, true)?;
        let p = s.probe("shop")?;
        assert_eq!(p["available"], true);
        assert_eq!(p["portable"], true);
        assert_eq!(
            p["evidence"]["response"]["body"][key][0][label],
            "Fresh food"
        );
        let mut no_export = deploy(&script.replace("data.json", "../escape"), cap, true)?;
        assert_eq!(no_export.probe("shop")?["portable"], false);
    }
    Ok(())
}

#[test]
fn invalid_deployments_and_special_files_fail_closed() -> Result<()> {
    let mut w = workspace::Workspace::default();
    w.write("handler.sh", "echo '{\"status\":200,\"body\":{}}'")?;
    let mut s = State::default();
    for entry in ["../handler.sh", "/etc/passwd", "missing", "."] {
        assert!(
            s.deploy(
                &w,
                json!({"id":"bad","entrypoint":entry,"capability":"directory","portable":false})
            )
            .is_err()
        );
    }
    let original = s.clone();
    w.write(
        "handler.sh",
        "/bin/busybox ln -s /etc/passwd link; echo '{\"status\":200,\"body\":{}}'",
    )?;
    assert!(
        s.deploy(
            &w,
            json!({"id":"bad","entrypoint":"handler.sh","capability":"directory","portable":false})
        )
        .is_err()
    );
    assert_eq!(s, original);
    assert!(deploy("echo not-json", "delivery", false).is_err());
    Ok(())
}
