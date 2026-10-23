import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout

path = Path(__file__).resolve().parents[1] / "sentinel_witness_gate.py"
spec = importlib.util.spec_from_file_location("witness", path)
wg = importlib.util.module_from_spec(spec)
spec.loader.exec_module(wg)


def resp(height=None, block_hash="x"):
    if height is None:
        return {"ok": False, "error_type": "TimeoutError"}
    return {"ok": True, "data": {"blockNumber": height, "blockHash": block_hash},
            "sha256": "a" * 64}


def sample(timestamp="2026-10-23T20:12:35+00:00", a=None, b=None, ah="a", bh="a"):
    return {"sample_utc": timestamp, "read_only": True,
            "sources": {"A": {"finality": resp(a, ah)},
                        "B": {"finality": resp(b, bh)}}}


def packet(*samples):
    return ("".join(json.dumps(s) + "\n" for s in samples)).encode()


def topology():
    return json.dumps({"schema": wg.TOPOLOGY_SCHEMA,
                       "validators": {"A": {"host": "first", "stake": 250},
                                      "D": {"host": "first", "stake": 250},
                                      "B": {"host": "second", "stake": 250},
                                      "C": {"host": "second", "stake": 250}}}).encode()


class GateTests(unittest.TestCase):
    def test_timeout_is_unobservable_not_process_down(self):
        report = wg.prepare_report(packet(sample()))
        self.assertEqual(report["analysis"]["assessment"], "INSUFFICIENT_FINALITY_OBSERVATION")
        self.assertFalse(report["analysis"]["protocol_finality_proven"])

    def test_agreement_not_finality_proof(self):
        report = wg.prepare_report(packet(sample(a=12, b=12)))
        self.assertEqual(report["analysis"]["assessment"], "OBSERVATIONS_ONLY_NO_FINALITY_PROOF")
        self.assertFalse(report["analysis"]["finalized_write_integrity_proven"])
        self.assertEqual(len(report["analysis"]["multi_source_reports"]), 1)

    def test_conflicting_reported_hash_at_same_height(self):
        report = wg.prepare_report(packet(sample(a=12, b=12, ah="a", bh="b")))
        self.assertEqual(report["analysis"]["assessment"], "REPORTED_CONFLICT_REVIEW_REQUIRED")
        self.assertIn("not an authenticated safety violation",
                      report["analysis"]["findings"][0]["boundary"])

    def test_different_heights_not_conflicting_state_witness(self):
        report = wg.prepare_report(packet(sample(a=12, b=13, ah="a", bh="b")))
        self.assertEqual(report["analysis"]["assessment"], "OBSERVATIONS_ONLY_NO_FINALITY_PROOF")

    def test_progress_and_nonprogress_are_observations(self):
        a=sample("2026-10-23T20:00:00+00:00", a=12, b=12)
        b=sample("2026-10-23T20:01:00+00:00", a=13, b=12)
        report=wg.prepare_report(packet(a,b))
        self.assertEqual(report["analysis"]["reported_finality_progress"]["A"]["status"], "REPORTED_PROGRESS")
        self.assertEqual(report["analysis"]["reported_finality_progress"]["B"]["status"], "NO_PROGRESS_OBSERVED")

    def test_regression_requires_review(self):
        a=sample("2026-10-23T20:00:00+00:00", a=13, b=12)
        b=sample("2026-10-23T20:01:00+00:00", a=12, b=12)
        report=wg.prepare_report(packet(a,b))
        self.assertEqual(report["analysis"]["assessment"], "REPORTED_CONFLICT_REVIEW_REQUIRED")

    def test_two_validators_one_host_quorum_hazard(self):
        report=wg.prepare_report(packet(sample()),topology())
        risk=report["analysis"]["host_failure_counterfactuals"]
        self.assertEqual(risk["failure_domains"],2)
        for case in risk["one_host_failure_scenarios"]:
            self.assertEqual(case["remaining_stake"],500)
            self.assertFalse(case["illustrative_gt_two_thirds_remaining"])

    def test_exact_two_thirds_after_failure_is_insufficient(self):
        validators = {"A": {"host": "one", "stake": 1},
                      "B": {"host": "two", "stake": 2}}
        scenarios = wg.failure_domains(validators)["one_host_failure_scenarios"]
        after_one = next(item for item in scenarios if item["host_failure"] == "one")
        self.assertEqual(after_one["remaining_stake"], 2)
        self.assertFalse(after_one["illustrative_gt_two_thirds_remaining"])

    def test_corrupted_report_nonfinite_number_blocks_cleanly(self):
        with tempfile.TemporaryDirectory() as dir:
            source = Path(dir) / "input.jsonl"
            source.write_bytes(packet(sample()))
            target = Path(dir) / "result.json"
            target.write_text('{"report_sha256": "a", "bad": NaN}')
            with redirect_stderr(io.StringIO()):
                self.assertEqual(wg.main([str(source), "--verify-report", str(target)]), 2)

    def test_omitted_topology_not_assumed(self):
        self.assertEqual(wg.prepare_report(packet(sample()))["analysis"]["host_failure_counterfactuals"]["status"],"NOT_PROVIDED")

    def test_low_memory_and_stale_health(self):
        s=sample()
        s["sources"]["A"]["health"]={"ok":True,"data":{"mem_available_mb":11,"ts":0}}
        report=wg.prepare_report(packet(s))
        self.assertEqual(report["analysis"]["low_memory_reports_at_or_below_16mb"][0]["available_memory_mb"],11)
        self.assertTrue(any(x["code"]=="HEALTH_SNAPSHOT_STALE_OR_FUTURE" for x in report["analysis"]["findings"]))

    def test_pin_mismatch_rejected(self):
        with self.assertRaisesRegex(wg.EvidenceError,"does not match"):
            wg.prepare_report(packet(sample()),expected="f"*64)

    def test_canonical_digest_and_matched_pin(self):
        raw=packet(sample())
        sha=hashlib.sha256(raw).hexdigest()
        a=wg.prepare_report(raw,topology(),sha)
        self.assertEqual(a,wg.prepare_report(raw,topology(),sha))
        recorded=a.pop("report_sha256")
        self.assertEqual(recorded,wg.digest(a))

    def test_duplicate_keys_rejected(self):
        bad=b'{"read_only":true,"read_only":true}\n'
        with self.assertRaisesRegex(wg.EvidenceError,"duplicate"):
            wg.parse_samples(bad)

    def test_read_only_required(self):
        item=sample()
        item["read_only"]=False
        with self.assertRaises(wg.EvidenceError):
            wg.parse_samples(packet(item))

    def test_non_finite_json_rejected(self):
        raw=packet(sample()).replace(b'"read_only": true',b'"read_only": NaN')
        with self.assertRaises(wg.EvidenceError):
            wg.parse_samples(raw)

    def test_timestamp_order_required(self):
        with self.assertRaisesRegex(wg.EvidenceError,"strictly increasing"):
            wg.parse_samples(packet(sample(),sample()))

    def test_malformed_successful_response_rejected(self):
        s=sample()
        s["sources"]["A"]["finality"]={"ok":True}
        with self.assertRaisesRegex(wg.EvidenceError,"without data"):
            wg.parse_samples(packet(s))

    def test_unsafe_topology_rejected(self):
        item=json.loads(topology())
        item["validators"]["A"]["stake"]=True
        with self.assertRaises(wg.EvidenceError):
            wg.topology_from_bytes(json.dumps(item).encode())

    def test_large_packet_rejected(self):
        with tempfile.TemporaryDirectory() as dir:
            p=Path(dir)/"large.jsonl"
            p.write_bytes(b"x"*(wg.MAX_INPUT+1))
            with self.assertRaises(wg.EvidenceError):
                wg.bounded_bytes(p)

    def test_verifier_recomputes_and_rejects_rehashed_forgery(self):
        with tempfile.TemporaryDirectory() as dir:
            capture = Path(dir) / "input.jsonl"
            capture.write_bytes(packet(sample(a=12, b=12)))
            report = Path(dir) / "report.json"
            with redirect_stdout(io.StringIO()):
                self.assertEqual(wg.main([str(capture), "--output", str(report)]), 0)
                self.assertEqual(wg.main([str(capture), "--verify-report", str(report)]), 0)
            forged = json.loads(report.read_text())
            forged["analysis"]["assessment"] = "FINALITY_PROVEN"
            body = dict(forged)
            body.pop("report_sha256")
            forged["report_sha256"] = wg.digest(body)
            report.write_text(json.dumps(forged))
            with redirect_stderr(io.StringIO()):
                self.assertEqual(wg.main([str(capture), "--verify-report", str(report)]), 2)

    def test_verifier_rejects_changed_raw_capture(self):
        with tempfile.TemporaryDirectory() as dir:
            capture = Path(dir) / "input.jsonl"
            capture.write_bytes(packet(sample(a=12, b=12)))
            report = Path(dir) / "report.json"
            with redirect_stdout(io.StringIO()):
                self.assertEqual(wg.main([str(capture), "--output", str(report)]), 0)
            capture.write_bytes(packet(sample(a=12, b=13)))
            with redirect_stderr(io.StringIO()):
                self.assertEqual(wg.main([str(capture), "--verify-report", str(report)]), 2)

    def test_cli_creates_once_and_refuses_overwrite(self):
        with tempfile.TemporaryDirectory() as dir:
            p=Path(dir)/"source.jsonl"
            p.write_bytes(packet(sample()))
            o=Path(dir)/"result.json"
            with redirect_stdout(io.StringIO()):
                self.assertEqual(wg.main([str(p),"--output",str(o)]),0)
            self.assertEqual(json.loads(o.read_text())["input"]["external_sha256_pin"],"NOT_PROVIDED")
            with redirect_stderr(io.StringIO()):
                self.assertEqual(wg.main([str(p),"--output",str(o)]),2)


if __name__=="__main__":
    unittest.main()
