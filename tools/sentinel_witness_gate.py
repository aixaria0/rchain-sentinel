#!/usr/bin/env python3
"""Offline, deterministic witness gate for read-only RChain incident captures.

Consumes JSONL produced by Rholo #421's incident-snapshot.py.
No requests, deploys, consensus actions, or production mutation are performed.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import sys

SCHEMA = "sentinel-incident-witness/v1"
TOPOLOGY_SCHEMA = "sentinel-observation-topology/v1"
MAX_INPUT = 5 * 1024 * 1024
MAX_SAMPLES = 100
HEX_SHA256 = re.compile(r"^[0-9a-f]{64}$")
LABEL = re.compile(r"^[A-Za-z][A-Za-z0-9_-]{0,31}$")


class EvidenceError(ValueError):
    """Invalid or untrusted input that cannot be interpreted safely."""


def unique_keys(pairs):
    out = {}
    for key, value in pairs:
        if key in out:
            raise EvidenceError(f"duplicate JSON key: {key}")
        out[key] = value
    return out


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=True,
                      separators=(",", ":"), allow_nan=False).encode("utf-8")


def digest(value):
    return hashlib.sha256(canonical(value)).hexdigest()


def bounded_bytes(path):
    with open(path, "rb") as source:
        data = source.read(MAX_INPUT + 1)
    if len(data) > MAX_INPUT:
        raise EvidenceError("capture exceeds 5 MiB")
    if not data:
        raise EvidenceError("capture is empty")
    return data


def utc_time(text):
    if not isinstance(text, str):
        raise EvidenceError("sample_utc must be a timestamp")
    try:
        value = dt.datetime.fromisoformat(text.replace("Z", "+00:00"))
    except ValueError as err:
        raise EvidenceError("invalid sample_utc") from err
    if value.tzinfo is None or value.utcoffset() != dt.timedelta(0):
        raise EvidenceError("sample_utc must be UTC")
    return value


def valid_reply(data, label, category):
    if not isinstance(data, dict) or type(data.get("ok")) is not bool:
        raise EvidenceError(f"{label}.{category} must be a response with boolean ok")
    sha = data.get("sha256")
    if sha is not None and (not isinstance(sha, str) or not HEX_SHA256.fullmatch(sha)):
        raise EvidenceError(f"{label}.{category} has an invalid body digest")
    if data["ok"] and not isinstance(data.get("data"), dict):
        raise EvidenceError(f"{label}.{category} has ok=true without data object")


def parse_samples(raw):
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as err:
        raise EvidenceError("capture is not UTF-8") from err
    rows = text.splitlines()
    if not (1 <= len(rows) <= MAX_SAMPLES):
        raise EvidenceError("capture requires 1..100 JSONL samples")
    parsed = []
    prior = None
    for index, row in enumerate(rows, start=1):
        if not row.strip():
            raise EvidenceError(f"blank sample at line {index}")
        try:
            entry = json.loads(row, object_pairs_hook=unique_keys,
                               parse_constant=lambda _: (_ for _ in ()).throw(
                                   EvidenceError("non-finite JSON constant")))
        except (json.JSONDecodeError, TypeError) as err:
            raise EvidenceError(f"invalid JSONL at line {index}") from err
        if not isinstance(entry, dict) or entry.get("read_only") is not True:
            raise EvidenceError(f"line {index}: expected read_only=true capture")
        at = utc_time(entry.get("sample_utc"))
        if prior is not None and at <= prior:
            raise EvidenceError("samples must have strictly increasing UTC timestamps")
        prior = at
        sources = entry.get("sources")
        if not isinstance(sources, dict) or not 1 <= len(sources) <= 50:
            raise EvidenceError("each sample requires 1..50 named sources")
        for label, categories in sources.items():
            if not LABEL.fullmatch(label) or not isinstance(categories, dict):
                raise EvidenceError("invalid source label or structure")
            if not categories or any(k not in ("status", "finality", "health")
                                      for k in categories):
                raise EvidenceError(f"{label}: unsupported or absent response kinds")
            for category, reply in categories.items():
                valid_reply(reply, label, category)
        parsed.append(entry)
    return parsed


def topology_from_bytes(raw):
    try:
        data = json.loads(raw, object_pairs_hook=unique_keys)
    except (UnicodeDecodeError, json.JSONDecodeError, TypeError) as err:
        raise EvidenceError("invalid topology JSON") from err
    if not isinstance(data, dict) or data.get("schema") != TOPOLOGY_SCHEMA:
        raise EvidenceError("unsupported topology schema")
    val = data.get("validators")
    if not isinstance(val, dict) or not 1 <= len(val) <= 100:
        raise EvidenceError("topology validators must be 1..100 entries")
    result = {}
    for label, item in val.items():
        if not LABEL.fullmatch(label) or not isinstance(item, dict):
            raise EvidenceError("invalid topology validator")
        if set(item) != {"host", "stake"}:
            raise EvidenceError("topology validator must declare host and stake only")
        host, stake = item["host"], item["stake"]
        if not isinstance(host, str) or not LABEL.fullmatch(host):
            raise EvidenceError("invalid failure-domain label")
        if type(stake) is not int or not 0 < stake <= 10 ** 18:
            raise EvidenceError("stake must be positive integer")
        result[label] = {"host": host, "stake": stake}
    return result


def sample_view(sample):
    at = utc_time(sample["sample_utc"])
    observations = {}
    for label, kinds in sorted(sample["sources"].items()):
        item = {"status_reachable": None, "finality_reachable": None,
                "health_reachable": None, "reported_tip": None,
                "reported_finalized_height": None,
                "reported_finalized_hash": None,
                "reported_available_memory_mb": None,
                "health_sample_age_seconds": None}
        for kind in ("status", "finality", "health"):
            response = kinds.get(kind)
            if response is None:
                continue
            item[f"{kind}_reachable"] = response["ok"]
            if not response["ok"]:
                continue
            body = response["data"]
            if kind == "status":
                tip = body.get("latestBlockNumber")
                if type(tip) is int and tip >= 0:
                    item["reported_tip"] = tip
            if kind == "finality":
                height = body.get("blockNumber")
                identity = body.get("blockHash")
                if type(height) is int and height >= 0:
                    item["reported_finalized_height"] = height
                if isinstance(identity, str) and 0 < len(identity) <= 256:
                    item["reported_finalized_hash"] = identity
            if kind == "health":
                free = body.get("mem_available_mb")
                if type(free) in (int, float) and 0 <= free < 10 ** 12:
                    item["reported_available_memory_mb"] = free
                stamp = body.get("ts")
                if type(stamp) in (int, float) and -2208988800 < stamp < 4102444800:
                    item["health_sample_age_seconds"] = round(
                        at.timestamp() - stamp, 3)
        observations[label] = item
    return {"utc": sample["sample_utc"], "sources": observations}


def failure_domains(validators):
    if not validators:
        return {"status": "NOT_PROVIDED", "source": "NO_VERIFIED_HOST_ATTESTATION"}
    total = sum(v["stake"] for v in validators.values())
    domains = sorted({v["host"] for v in validators.values()})
    scenarios = []
    for host in domains:
        affected = sorted(label for label, v in validators.items()
                          if v["host"] == host)
        lost = sum(validators[label]["stake"] for label in affected)
        remaining = total - lost
        scenarios.append({
            "host_failure": host, "affected_validators": affected,
            "lost_stake": lost, "remaining_stake": remaining,
            "illustrative_gt_two_thirds_remaining": 3 * remaining > 2 * total,
        })
    return {
        "status": "ILLUSTRATIVE_ONLY",
        "basis": "CALLER_SUPPLIED_UNATTESTED_TOPOLOGY",
        "total_configured_stake": total,
        "failure_domains": len(domains),
        "one_host_failure_scenarios": scenarios,
        "boundary": "Not a Casper CBC finality certificate or verified network bond set.",
    }


def analyze(samples, validators=None):
    views = [sample_view(entry) for entry in samples]
    identities = []
    problems = []
    by_validator = {}
    missing = 0
    mem_low = []
    for snap in views:
        height_to_hashes = {}
        for label, obs in snap["sources"].items():
            height, identity = (obs["reported_finalized_height"],
                                obs["reported_finalized_hash"])
            if height is not None:
                by_validator.setdefault(label, []).append((snap["utc"], height))
            if height is not None and identity is not None:
                height_to_hashes.setdefault(height, {}).setdefault(identity, []).append(label)
            if obs["finality_reachable"] is not True:
                missing += 1
            age = obs["health_sample_age_seconds"]
            if age is not None and (age > 120 or age < -30):
                problems.append({
                    "code": "HEALTH_SNAPSHOT_STALE_OR_FUTURE",
                    "utc": snap["utc"], "source": label,
                    "age_seconds": age,
                })
            free = obs["reported_available_memory_mb"]
            if free is not None and free <= 16:
                mem_low.append({"utc": snap["utc"], "source": label,
                                "available_memory_mb": free})
        for height, hashes in height_to_hashes.items():
            if len(hashes) > 1:
                problems.append({
                    "code": "REPORTED_FINALIZED_IDENTITY_CONFLICT",
                    "utc": snap["utc"], "height": height,
                    "source_groups": [
                        {"reported_hash": identity, "sources": sorted(labels)}
                        for identity, labels in sorted(hashes.items())
                    ],
                    "boundary": "Conflicting reported hashes, not an authenticated safety violation.",
                })
            else:
                only = next(iter(hashes.items()))
                if len(only[1]) > 1:
                    identities.append({
                        "utc": snap["utc"], "height": height,
                        "reported_hash": only[0],
                        "sources": sorted(only[1]),
                        "boundary": "Apparent agreement; source independence unverified.",
                    })
    progress = {}
    for label, points in sorted(by_validator.items()):
        heights = [value for _, value in points]
        if len(heights) < 2:
            state = "ONE_OBSERVATION_ONLY"
        elif any(b < a for a, b in zip(heights, heights[1:])):
            state = "REPORTED_REGRESSION_REQUIRES_REVIEW"
            problems.append({"code": "REPORTED_FINALIZED_HEIGHT_REGRESSION", "source": label})
        elif heights[-1] > heights[0]:
            state = "REPORTED_PROGRESS"
        else:
            state = "NO_PROGRESS_OBSERVED"
        progress[label] = {"status": state, "observations": len(points),
                           "first_height": heights[0], "last_height": heights[-1]}
    conflict = any(p["code"] in ("REPORTED_FINALIZED_IDENTITY_CONFLICT",
                                 "REPORTED_FINALIZED_HEIGHT_REGRESSION")
                   for p in problems)
    verdict = ("REPORTED_CONFLICT_REVIEW_REQUIRED" if conflict
               else "INSUFFICIENT_FINALITY_OBSERVATION" if missing
               else "OBSERVATIONS_ONLY_NO_FINALITY_PROOF")
    return {
        "samples": views, "multi_source_reports": identities,
        "reported_finality_progress": progress,
        "low_memory_reports_at_or_below_16mb": mem_low,
        "unanswered_finality_requests": missing,
        "findings": problems,
        "host_failure_counterfactuals": failure_domains(validators),
        "assessment": verdict,
        "protocol_finality_proven": False,
        "finalized_write_integrity_proven": False,
        "boundaries": [
            "Request failure means unobservable from this client, not a proven stopped process.",
            "A reported finalized block is not an independently certified finality proof.",
            "Source labels and distinct HTTP endpoints do not attest independent hosts or operators.",
            "Two differing tips alone are not evidence of a finalized fork.",
            "An executed deploy and a finalized state mutation are separate claims.",
            "No kernel OOM conclusion can be drawn from available-memory or HTTP readings alone.",
        ],
    }


def prepare_report(raw, topology=None, expected=None):
    source_sha = hashlib.sha256(raw).hexdigest()
    if expected is not None:
        if not HEX_SHA256.fullmatch(expected) or expected != source_sha:
            raise EvidenceError("external input SHA-256 does not match")
    samples = parse_samples(raw)
    validators = topology_from_bytes(topology) if topology is not None else None
    body = {
        "schema": SCHEMA, "input": {"sha256": source_sha, "sample_count": len(samples),
                                    "external_sha256_pin": "MATCHED" if expected else "NOT_PROVIDED"},
        "topology_sha256": hashlib.sha256(topology).hexdigest() if topology else None,
        "analysis": analyze(samples, validators),
        "trust": {
            "input_provenance": "CALLER_SUPPLIED_BYTES",
            "network_access": "NONE",
            "source_independence": "UNATTESTED",
            "provenance_authenticated": False,
            "report_digest_authenticates_origin": False,
        },
    }
    body["report_sha256"] = digest(body)
    return body


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture_jsonl", help="Original #421 read-only incident snapshot JSONL")
    parser.add_argument("--topology", help="Optional illustrative host/stake map JSON")
    parser.add_argument("--expect-sha256", help="Externally reviewed raw JSONL SHA-256")
    parser.add_argument("--output", help="New JSON result file (refuses overwrite)")
    parser.add_argument("--verify-report", help="Recompute and check an existing report against the supplied raw captures")
    args = parser.parse_args(argv)
    try:
        raw = bounded_bytes(args.capture_jsonl)
        topo = bounded_bytes(args.topology) if args.topology else None
        report = prepare_report(raw, topo, args.expect_sha256)
        encoded = json.dumps(report, indent=2, sort_keys=True, ensure_ascii=True) + "\n"
        if args.verify_report:
            if args.output:
                raise EvidenceError("--output and --verify-report are mutually exclusive")
            received = json.loads(bounded_bytes(args.verify_report), object_pairs_hook=unique_keys)
            if not isinstance(received, dict):
                raise EvidenceError("report must be a JSON object")
            recorded_digest = received.get("report_sha256")
            body = dict(received)
            body.pop("report_sha256", None)
            if recorded_digest != digest(body):
                raise EvidenceError("report body digest mismatch")
            if canonical(received) != canonical(report):
                raise EvidenceError("report differs from recomputation over pinned input")
            print("VERIFIED_RECOMPUTED_ARTIFACT_ONLY; network finality NOT_ESTABLISHED")
            return 0
        if args.output:
            output = Path(args.output)
            if output.exists():
                raise EvidenceError("output already exists; refusing overwrite")
            # Exclusive create: never replace another file.
            with output.open("x", encoding="utf-8") as stream:
                stream.write(encoded)
                stream.flush()
                os.fsync(stream.fileno())
            print(f"written {output} assessment={report['analysis']['assessment']}")
        else:
            sys.stdout.write(encoded)
        return 0
    except (OSError, ValueError, json.JSONDecodeError) as err:
        print(f"witness gate blocked: {err}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
