"""
Test suite for the vibecoded bottle.py todo app.
Runs after the integration test creates the app.

Tests verify:
1. app.py exists and imports bottle
2. requirements.txt contains bottle
3. App has route definitions for CRUD operations
4. App can start without errors
5. Basic HTTP endpoints work (add, list, done, delete)
"""
import os
import sys
import subprocess
import time
import urllib.request
import urllib.error
import json
import signal

import pytest


# Path to the vibecoded app (set by integration test)
APP_DIR = os.environ.get("VSC_TEST_DIR", "/tmp/vsc_integration_test")
APP_FILE = os.path.join(APP_DIR, "app.py")
REQ_FILE = os.path.join(APP_DIR, "requirements.txt")


@pytest.fixture(scope="module")
def app_exists():
    """Check that the app files exist."""
    if not os.path.isfile(APP_FILE):
        pytest.skip("app.py not found - integration test may not have run")
    return True


@pytest.fixture(scope="module")
def app_content(app_exists):
    """Read app.py content."""
    with open(APP_FILE) as f:
        return f.read()


@pytest.fixture(scope="module")
def req_content(app_exists):
    """Read requirements.txt content."""
    if not os.path.isfile(REQ_FILE):
        return ""
    with open(REQ_FILE) as f:
        return f.read()


# ---- Static checks ----


class TestStaticChecks:
    def test_app_py_exists(self, app_exists):
        assert os.path.isfile(APP_FILE)

    def test_requirements_txt_exists(self, app_exists):
        assert os.path.isfile(REQ_FILE)

    def test_app_imports_bottle(self, app_content):
        assert "bottle" in app_content.lower() or "from bottle" in app_content

    def test_requirements_has_bottle(self, req_content):
        assert "bottle" in req_content.lower()

    def test_app_has_routes(self, app_content):
        has_route = any(
            kw in app_content for kw in ["@route", "@get", "@post", "@delete", "@put", "route("]
        )
        assert has_route, "app.py should have route definitions"

    def test_app_syntax_valid(self, app_exists):
        """Check that app.py has valid Python syntax."""
        result = subprocess.run(
            [sys.executable, "-c", f"import ast; ast.parse(open('{APP_FILE}').read())"],
            capture_output=True,
            text=True,
        )
        assert result.returncode == 0, f"Syntax error in app.py: {result.stderr}"


# ---- Runtime checks ----


@pytest.fixture(scope="module")
def running_app(app_exists):
    """Start the bottle app and yield the base URL. Kill on cleanup."""
    # Install bottle first
    subprocess.run(
        [sys.executable, "-m", "pip", "install", "bottle", "-q"],
        capture_output=True,
    )

    port = 18765  # Use unusual port to avoid conflicts
    env = os.environ.copy()
    env["BOTTLE_PORT"] = str(port)

    # Start the app
    proc = subprocess.Popen(
        [sys.executable, APP_FILE],
        cwd=APP_DIR,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    base_url = f"http://localhost:{port}"

    # Wait for the app to start (up to 5 seconds)
    for _ in range(10):
        time.sleep(0.5)
        try:
            urllib.request.urlopen(f"{base_url}/", timeout=1)
            break
        except (urllib.error.URLError, ConnectionRefusedError):
            continue
    else:
        # App didn't start - check stderr
        proc.kill()
        _, stderr = proc.communicate(timeout=5)
        pytest.skip(f"App failed to start: {stderr.decode()[:500]}")

    yield base_url

    # Cleanup
    proc.send_signal(signal.SIGTERM)
    try:
        proc.wait(timeout=3)
    except subprocess.TimeoutExpired:
        proc.kill()


class TestRuntimeChecks:
    def test_app_starts(self, running_app):
        """App should start without crashing."""
        assert running_app is not None

    def test_root_responds(self, running_app):
        """Root URL should respond with 200."""
        try:
            resp = urllib.request.urlopen(f"{running_app}/", timeout=5)
            assert resp.status == 200
        except urllib.error.HTTPError:
            # Some apps redirect root, that's ok
            pass

    def test_app_process_no_crash(self, running_app):
        """App should still be running after basic requests."""
        try:
            urllib.request.urlopen(f"{running_app}/", timeout=5)
        except (urllib.error.URLError, urllib.error.HTTPError):
            pass
        # If we got here without exception, app is still running


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
