"""Native product checks for executable-local discovery and numeric failure policy."""
import argparse
import json
import os
import pathlib
import shutil
import subprocess
import tempfile


def run(binary):
    records = []
    with tempfile.TemporaryDirectory(prefix="rt-runtime-") as temporary:
        root = pathlib.Path(temporary)
        ambient = root / "ambient"
        ambient.mkdir()
        for name in ("conpty.dll", "OpenConsole.exe"):
            shutil.copyfile(binary.parent / name, ambient / name)
        environment = os.environ.copy()
        for name in ("CI", "RELAYTERM_TEST_RT", "RELAYTERM_EVALUATION_CONPTY", "RELAYTERM_READER_METRICS", "RELAYTERM_READER_DELAY_US"):
            environment.pop(name, None)
        environment["PATH"] = str(ambient) + os.pathsep + str(pathlib.Path(os.environ["SystemRoot"]) / "System32")
        for case in ("missing-dll", "missing-helper", "wrong-architecture", "altered-dll", "altered-helper", "valid-anchored"):
            candidate = root / case
            candidate.mkdir()
            shutil.copyfile(binary, candidate / "rt.exe")
            for name in ("conpty.dll", "OpenConsole.exe"):
                shutil.copyfile(binary.parent / name, candidate / name)
            expected = ""
            if case.startswith("missing"):
                (candidate / ("conpty.dll" if case == "missing-dll" else "OpenConsole.exe")).unlink()
                expected = "os error 2"
            elif case == "wrong-architecture":
                value = bytearray((candidate / "conpty.dll").read_bytes())
                offset = int.from_bytes(value[60:64], "little")
                value[offset + 4:offset + 6] = b"\x4c\x01"
                (candidate / "conpty.dll").write_bytes(value)
                expected = "code=193"
            elif case.startswith("altered"):
                path = candidate / ("conpty.dll" if case == "altered-dll" else "OpenConsole.exe")
                with path.open("ab") as output:
                    output.write(b"synthetic-tamper")
                expected = "code=13"
            # Valid ambient files must not rescue an incomplete executable directory.
            environment["RELAYTERM_EVALUATION_CONPTY"] = str(ambient / "conpty.dll")
            process = subprocess.run([str(candidate / "rt.exe"), "--format", "json", "--workspace", str(ambient),
                "--home", str(root / (case + "-state")), "workspace", "status"],
                cwd=ambient, env=environment, capture_output=True, text=True, timeout=30)
            value = json.loads(process.stdout)
            if expected:
                assert process.returncode == 3 and value["error"]["code"] == "runtime_unavailable", (case, value)
                assert expected in value["error"]["message"], (case, value)
                assert not (root / (case + "-state")).exists(), "runtime failure caused state mutation"
            else:
                assert value.get("error", {}).get("code") != "runtime_unavailable", value
            records.append(dict(case=case, exit=process.returncode, expected_numeric=expected, runtime_accepted=not expected))
    print(json.dumps(records, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=pathlib.Path)
    run(parser.parse_args().binary.resolve())
