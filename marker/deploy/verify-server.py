#!/usr/bin/env python3
"""Bounded CLI/server smoke. Starts and reaps only its own loopback child."""
import argparse
import hashlib
import http.client
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--web-dir", type=Path, required=True)
    args = parser.parse_args()
    binary = args.binary.resolve()
    env = {k: v for k, v in os.environ.items() if k != "BETA_TOKEN"}
    prefix = ["nice", "-n", "10"] if os.name != "nt" else []
    help_result = subprocess.run(prefix + [str(binary), "--help"], env=env, capture_output=True, text=True, timeout=10)
    assert help_result.returncode == 0, help_result.stderr
    version_result = subprocess.run(prefix + [str(binary), "--version"], env=env, capture_output=True, text=True, timeout=10)
    assert version_result.returncode == 0 and "0.5.0" in version_result.stdout, version_result.stdout
    rejected = subprocess.run(prefix + [str(binary), "serve", "--bind", "0.0.0.0"], env=env, capture_output=True, text=True, timeout=10)
    assert rejected.returncode != 0 and "requires --token or BETA_TOKEN" in rejected.stderr, rejected.stderr
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]
    token = secrets.token_hex(32)
    env["BETA_TOKEN"] = token
    child = subprocess.Popen(prefix + [str(binary), "serve", "--bind", "127.0.0.1", "--port", str(port), "--web-dir", str(args.web_dir.resolve())], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

    def request(path, headers=None):
        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=3)
        try:
            connection.request("GET", path, headers=headers or {})
            response = connection.getresponse()
            return response.status, response.read()
        finally:
            connection.close()

    try:
        for _ in range(50):
            try:
                status, _ = request("/api/health")
                break
            except OSError:
                if child.poll() is not None:
                    raise RuntimeError("Server exited before startup")
                time.sleep(0.1)
        else:
            raise RuntimeError("Server did not start within five seconds")
        assert status == 401
        assert request("/api/nest/status")[0] == 401
        assert request("/api/health", {"Authorization": "Bearer wrong"})[0] == 401
        assert request("/api/health?token=" + token)[0] == 401
        status, body = request("/api/health", {"Authorization": "Bearer " + token})
        health = json.loads(body)
        assert status == 200 and health["ok"] and health["version"] == "0.5.0", health
        assert request("/")[0] == 200
        assert request("/%2e%2e/Cargo.toml")[0] == 404
        print("PASS: help/version; unauthenticated network-bind rejection; token from BETA_TOKEN; health/status authentication; static index; traversal rejection")
    finally:
        child.terminate()
        try:
            stdout, stderr = child.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            child.kill()
            stdout, stderr = child.communicate(timeout=5)
        assert token not in stdout + stderr, "Token leaked into server logs"
        print("PASS: temporary loopback server stopped and reaped")
    print(json.dumps({"binary": binary.name, "bytes": binary.stat().st_size,
                      "sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}))


if __name__ == "__main__":
    main()
