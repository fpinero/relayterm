"""Negative controls for independent sustained evidence arithmetic."""
import importlib.util
import pathlib
import unittest

spec = importlib.util.spec_from_file_location("verifier", pathlib.Path(__file__).with_name("verify_sustained_evidence.py"))
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)


class VerifierTests(unittest.TestCase):
    def fixture(self, rate=3 * 1024 * 1024, echo=1000):
        points = "\n".join(f"M12 load {s * 1000000} {s * 1000000 + 1000} {s * rate} {s * 100}" for s in range(123))
        samples = [echo] * 100
        return ("M12 load points begin_us end_us consumed_flood consumed_screen\n" + points
            + f"\nM12 raw navigation latency_us={[1000] * 100}\nM12 raw echo latency_us={samples}\n"
            + "\n".join(f"M12 sustained {name} start_ms=0 end_ms=123000" for name in ("entire-load", "navigation", "echo")))

    def test_full_contract_and_exact_interval_arithmetic(self):
        result = verifier.recompute(self.fixture())[0]
        self.assertTrue(result["qualified"])
        self.assertEqual(result["intervals"][0]["rate_bytes_s"], 3145728 * 1000000 // 1001000)
        self.assertEqual(result["latencies"]["echo"]["p95_us"], 1000)

    def test_slow_load_cannot_become_latency_acceptance(self):
        self.assertFalse(verifier.recompute(self.fixture(rate=1024 * 1024))[0]["qualified"])

    def test_echo_budget_is_not_relaxed(self):
        self.assertFalse(verifier.recompute(self.fixture(echo=250001))[0]["qualified"])

    def test_stopped_screen_and_incomplete_samples_fail(self):
        text = self.fixture().replace("M12 load 1000000 1001000 3145728 100", "M12 load 1000000 1001000 3145728 0")
        self.assertFalse(verifier.recompute(text)[0]["qualified"])
        self.assertFalse(verifier.recompute("M12 load points begin_us end_us consumed_flood consumed_screen\n")[0]["qualified"])


if __name__ == "__main__":
    unittest.main()
