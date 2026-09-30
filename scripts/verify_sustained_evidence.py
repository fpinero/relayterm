"""Recompute sustained observations without changing acceptance thresholds."""
import argparse
import ast
import hashlib
import json
import pathlib
import re

MIN_RATE = 2 * 1024 * 1024
MAX_GAP_US = 3_000_000


def rate(first, last):
    elapsed = last[1] - first[0]
    delta = last[2] - first[2]
    return delta * 1_000_000 // elapsed if elapsed > 0 and delta >= 0 else None


def recompute(text):
    results = []
    for section in text.split("M12 load points begin_us end_us consumed_flood consumed_screen")[1:]:
        points = [list(map(int, match)) for match in re.findall(r"^M12 load (\d+) (\d+) (\d+) (\d+)$", section, re.M)]
        intervals = [{"elapsed_us": b[1] - a[0], "flood_delta": b[2] - a[2],
                      "screen_delta": b[3] - a[3], "rate_bytes_s": rate(a, b),
                      "qualified": b[1] - a[0] <= MAX_GAP_US and b[3] > a[3]
                      and rate(a, b) is not None and rate(a, b) >= MIN_RATE}
                     for a, b in zip(points, points[1:])]
        windows = []
        for name, start, end in re.findall(r"M12 sustained (entire-load|navigation|echo) start_ms=(\d+) end_ms=(\d+)", section):
            start, end = int(start) * 1000, int(end) * 1000
            # Printed boundaries are truncated to milliseconds. Preserve that
            # precision boundary explicitly instead of inventing nanoseconds.
            selected = [p for p in points if p[0] >= start + 1000 and p[1] <= end]
            qualified = len(selected) >= 2 and selected[0][0] - start <= MAX_GAP_US and end - selected[-1][1] <= MAX_GAP_US
            qualified = qualified and all(b[3] > a[3] and b[1] - a[0] <= MAX_GAP_US
                and rate(a, b) is not None and rate(a, b) >= MIN_RATE for a, b in zip(selected, selected[1:]))
            measured = rate(selected[0], selected[-1]) if len(selected) >= 2 else None
            qualified = qualified and measured is not None and measured >= MIN_RATE
            windows.append(dict(name=name, start_ms=start // 1000, end_ms=end // 1000,
                selected_count=len(selected), rate_bytes_s=measured, qualified=qualified,
                boundary_precision="conservative interior of truncated millisecond boundaries"))
        latencies = {}
        for name, raw in re.findall(r"M12 raw (navigation|echo) latency_us=(\[[^\n]+\])", section):
            samples = ast.literal_eval(raw)
            ordered = sorted(samples)
            p95 = ordered[(len(ordered) * 95 + 99) // 100 - 1] if ordered else None
            latencies[name] = dict(samples_us=samples, count=len(samples), p95_us=p95,
                maximum_us=max(samples, default=None), qualified=len(samples) == 100
                and p95 <= (100_000 if name == "navigation" else 250_000))
        producers = {name: dict(elapsed_us=int(elapsed), bytes=int(count), frames=int(frames),
                               rate_bytes_s=int(count) * 1_000_000 // int(elapsed))
            for name, elapsed, count, frames in re.findall(r"M12 producer (flood|screen) elapsed_us bytes frames=(\d+) (\d+) (\d+)", section)}
        span = points[-1][1] - points[0][0] if points else 0
        results.append(dict(points=points, intervals=intervals, windows=windows, latencies=latencies,
            producers=producers, span_us=span, timestamp_precision_us=1,
            arithmetic_note="Rates are independently recomputed from printed microseconds; native assertions use nanoseconds.",
            whole_span_rate_bytes_s=rate(points[0], points[-1]) if len(points) >= 2 else None,
            qualified=span >= 120_000_000 and bool(intervals) and all(i["qualified"] for i in intervals)
                and len(windows) == 3 and all(w["qualified"] for w in windows)
                and len(latencies) == 2 and all(v["qualified"] for v in latencies.values())))
    return results


def resources(text):
    helpers = [{"helpers": ast.literal_eval(raw), "daemon_bytes": int(daemon), "aggregate_bytes": int(total)}
        for raw, daemon, total in re.findall(r"M12 runtime helpers=(\[[^\n]+\]) daemon_bytes=(\d+) aggregate_bytes=(\d+)", text)]
    samples = [dict(zip(("elapsed_ms", "daemon_bytes", "tui_bytes", "fixture_bytes"), map(int, values)))
        for values in re.findall(r"M12 resource elapsed_ms=(\d+) daemon_bytes=(\d+) tui_bytes=(\d+) fixture_bytes=(\d+)", text)]
    aggregate = [item["aggregate_bytes"] for item in helpers]
    steady = aggregate[min(20, len(aggregate)):]
    size = min(10, len(steady) // 2)
    median = lambda values: sorted(values)[len(values) // 2] if values else None
    first, last = median(steady[:size]), median(steady[-size:])
    return dict(helper_observations=helpers, process_samples=samples,
        maximum_owned_daemon_bytes=max(aggregate, default=None),
        first_steady_median_bytes=first, last_steady_median_bytes=last,
        qualified_memory=len(aggregate) >= 40 and max(aggregate) <= 512 * 1024 * 1024
            and first is not None and last <= first + 32 * 1024 * 1024,
        handle_counts=[list(map(int, pair)) for pair in re.findall(r"M12 runtime aggregate_handles_baseline=(\d+) aggregate_handles_final=(\d+)", text)],
        lifecycle=dict(re.findall(r"M12 runtime (owned_helpers_\w+)=(\d+)", text)),
        module_loaded_verified="M12 runtime loaded_module_verified=true" in text)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("logs", nargs="+", type=pathlib.Path)
    parser.add_argument("--output", required=True, type=pathlib.Path)
    args = parser.parse_args()
    records = [{"attempt": path.parent.name, "log": path.name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                "observations": recompute(path.read_text(encoding="utf-8", errors="replace")),
                "resources": resources(path.read_text(encoding="utf-8", errors="replace"))}
               for path in args.logs]
    args.output.write_text(json.dumps({"minimum_bytes_s": MIN_RATE, "max_gap_us": MAX_GAP_US,
        "records": records}, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"logs": len(records), "scenarios": sum(len(r["observations"]) for r in records),
        "qualified": sum(s["qualified"] for r in records for s in r["observations"])}))
