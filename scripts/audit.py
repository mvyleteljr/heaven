#!/usr/bin/env python3
"""Run an explicitly budgeted suite against local Frontier; no inference retries.

Each run and its effective configuration are saved before starting it. Reports
are replaced atomically after every status change. Re-running with the same
output path is refused, avoiding duplicate paid runs after an unknown outcome.
"""
import argparse
import datetime
import hashlib
import json
import math
from pathlib import Path
import time
import urllib.error
import urllib.parse
import urllib.request


def validate(spec):
    if spec.get("version") != 1 or not spec.get("cases"):
        raise ValueError("Expected version 1 and nonempty cases")
    budget = spec.get("budget_usd")
    if not isinstance(budget, (int, float)) or not math.isfinite(budget) or budget <= 0:
        raise ValueError("budget_usd must be positive and finite")
    total = 0
    names = set()
    for case in spec["cases"]:
        if case["endpoint"] not in ["recall", "records", "flowers", "bellwether"]:
            raise ValueError("Unknown fixture endpoint")
        if case["name"] in names:
            raise ValueError("Case names must be unique")
        names.add(case["name"])
        config = case["config"]
        limit = config["dollar_limit"]
        if not isinstance(limit, (int, float)) or not math.isfinite(limit) or limit <= 0:
            raise ValueError("Invalid case dollar limit")
        if config.get("demo") is not False:
            raise ValueError("This runner measures explicit live conditions")
        total += limit
    if total > budget + 1e-9:
        raise ValueError("Sum of case ceilings exceeds suite budget")
    return total


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("suite", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--base-url", default="http://127.0.0.1:3000")
    parser.add_argument("--timeout-seconds", type=int, default=1800)
    args = parser.parse_args()
    parsed = urllib.parse.urlparse(args.base_url)
    if parsed.scheme != "http" or parsed.hostname not in ["127.0.0.1", "localhost", "::1"] or parsed.username or parsed.query or parsed.fragment or parsed.path not in ["", "/"]:
        parser.error("Use a loopback HTTP server URL")
    if args.timeout_seconds < 1:
        parser.error("Timeout must be positive")
    source = args.suite.read_bytes()
    spec = json.loads(source)
    validate(spec)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    # Exclusive reservation: never overwrite or automatically resume a suite.
    with args.output.open("x") as stream:
        stream.write("{}\n")
    report = {"version": 1, "suite": spec, "suite_sha256": hashlib.sha256(source).hexdigest(), "started_at": datetime.datetime.now(datetime.timezone.utc).isoformat(), "status": "starting", "cases": []}

    def save():
        temporary = args.output.with_suffix(args.output.suffix + ".tmp")
        temporary.write_text(json.dumps(report, indent=2) + "\n")
        temporary.replace(args.output)

    def api(path, body=None):
        request = urllib.request.Request(args.base_url.rstrip("/") + "/api/" + path, data=None if body is None else json.dumps(body).encode(), headers={"Content-Type": "application/json", "X-Frontier-Client": "research-ui"})
        try:
            with urllib.request.urlopen(request, timeout=120) as response:
                return json.load(response)
        except urllib.error.HTTPError as error:
            raise RuntimeError(f"HTTP {error.code}: {error.read(4096).decode(errors='replace')}") from error

    active_id = None
    try:
        health = api("health")
        report["source_sha256"] = health["source_sha256"]
        if not health["key_available"]:
            raise RuntimeError("Server has no inference key")
        if any(r["status"] == "running" for r in api("runs")):
            raise RuntimeError("Another run is active; suite was not started")
        report["status"] = "running"
        save()
        for case in spec["cases"]:
            if api("health")["source_sha256"] != report["source_sha256"]:
                raise RuntimeError("Server source changed; refusing mixed-version audit")
            current = {"name": case["name"], "status": "creating"}
            report["cases"].append(current)
            save()
            run = api(case["endpoint"], {**case["config"], "name": case["name"]})
            active_id = run["id"]
            current.update({"id": active_id, "config": run["config"], "status": "created"})
            save()
            api(f"runs/{active_id}/control", {"command": "run"})
            deadline = time.monotonic() + args.timeout_seconds
            last = None
            while True:
                audit = api(f"runs/{active_id}/audit")
                if audit["reporting_source_sha256"] != report["source_sha256"]:
                    raise RuntimeError("Server source changed during run")
                progress = (audit["status"], audit["integrity"]["attempted_actions"], audit["spending"]["calls_including_inherited"])
                if progress != last:
                    current.update({"status": audit["status"], "audit": audit})
                    save()
                    print(json.dumps({"case": case["name"], "status": progress[0], "actions": progress[1], "calls": progress[2]}), flush=True)
                    last = progress
                if not audit["busy"] and audit["status"] != "running":
                    break
                if time.monotonic() >= deadline:
                    raise TimeoutError("Case wall-clock limit reached; pause requested")
                time.sleep(3)
            active_id = None
            # Provider/preflight failure is shared infrastructure, not an excuse
            # to spend on the next condition or retry an uncertain request.
            if any(e["kind"] == "actor_rejected" or (e["kind"] == "inference_failure" and e.get("phase") != "preflight") for e in audit["failures"]):
                report["status"] = "blocked_by_inference"
                break
        else:
            report["status"] = "completed"
    except BaseException as error:
        report["status"] = "interrupted"
        report["error"] = str(error)
        if active_id:
            try:
                api(f"runs/{active_id}/control", {"command": "pause"})
                report["pause_requested_for"] = active_id
            except Exception as pause_error:
                report["pause_error"] = str(pause_error)
        raise
    finally:
        report["recorded_charged_usd"] = sum(c.get("audit", {}).get("spending", {}).get("charged_usd", 0) for c in report["cases"])
        report["recorded_uncertain_reserved_usd"] = sum(c.get("audit", {}).get("spending", {}).get("uncertain_reserved_usd", 0) for c in report["cases"])
        report["note"] = "Recorded totals can lag an interrupted in-flight request. Case ceilings remain authoritative. Task failure is not harness failure; no automatic retries."
        save()


if __name__ == "__main__":
    main()
