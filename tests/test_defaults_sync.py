"""Run with python3 tests/test_defaults_sync.py; touches only a temporary home."""
import json
import os
from pathlib import Path
import subprocess
import tempfile


with tempfile.TemporaryDirectory() as temporary:
    home = Path(temporary)
    source = home / ".claude"
    profile = home / "accounts/profiles/work"
    source.mkdir()
    profile.mkdir(parents=True)
    original = {"env": {"ANTHROPIC_API_KEY": "test-key"}, "model": "custom",
                "enabledPlugins": {"ponytail@ponytail": False},
                "hooks": {"PreToolUse": [{"hooks": [{"command": "existing-hook"}]}]}}
    (profile / "settings.json").write_text(json.dumps(original))
    (profile / "CLAUDE.md").write_text("Existing instructions\n")
    (source / "settings.json").write_text('{}')
    (source / "RTK.md").write_text("Shared RTK instructions\n")
    (source / ".i-have-adhd-always").touch()
    (home / "accounts/state.json").write_text(json.dumps({
        "profiles": {"work": {"config_dir": str(profile)}}}))
    script = Path(__file__).resolve().parents[1] / "scripts/claude-defaults-sync.sh"
    environment = dict(os.environ, HOME=str(home), CLAUDE_ACCOUNT_HOME=str(home / "accounts"))
    subprocess.run(["sh", str(script)], env=environment, check=True)
    settings = json.loads((profile / "settings.json").read_text())
    assert settings["env"] == original["env"]
    assert settings["model"] == "custom"
    assert settings["enabledPlugins"]["ponytail@ponytail"] is False
    assert settings["enabledPlugins"]["i-have-adhd@i-have-adhd"] is True
    assert settings["hooks"]["PreToolUse"][0] == original["hooks"]["PreToolUse"][0]
    assert (profile / "RTK.md").read_text() == (source / "RTK.md").read_text()
    assert (profile / "CLAUDE.md").read_text() == "@RTK.md\nExisting instructions\n"
    tracked = [p for p in profile.rglob("*") if p.is_file()]
    before = {str(p): (p.read_bytes(), p.stat().st_mtime_ns) for p in tracked}
    subprocess.run(["sh", str(script)], env=environment, check=True)
    assert before == {str(p): (p.read_bytes(), p.stat().st_mtime_ns) for p in tracked}
    print("Defaults sync: preserves settings and is idempotent")
