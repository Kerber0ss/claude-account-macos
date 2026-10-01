#!/bin/sh
# Add shared defaults without replacing profile-specific settings.
exec python3 - "$@" <<'PY'
import fcntl
import json
import os
from pathlib import Path
import sys
import tempfile


def read_json(path, default):
    if path.is_symlink():
        raise ValueError(f"refusing to edit symlink: {path}")
    return json.loads(path.read_text()) if path.exists() else default


def save_json(path, value):
    if read_json(path, None) == value:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(dir=path.parent, prefix=path.name + ".tmp.")
    try:
        with os.fdopen(fd, "w") as output:
            json.dump(value, output, ensure_ascii=False, indent=2)
            output.write("\n")
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def link_missing(source, destination):
    if source.exists() and not os.path.lexists(destination):
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.symlink_to(source, target_is_directory=source.is_dir())


def sync():
    home = Path.home()
    source = home / ".claude"
    root = Path(os.environ.get("CLAUDE_ACCOUNT_HOME", str(Path(os.environ.get("XDG_DATA_HOME", str(home / "Library/Application Support"))) / "claude-account")))
    config_root = Path(os.environ.get("CLAUDE_ACCOUNT_HOME", str(Path(os.environ.get("XDG_CONFIG_HOME", str(home / "Library/Application Support"))) / "claude-account")))
    with (config_root / "state.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        state = read_json(config_root / "state.json", {})
        defaults = read_json(source / "settings.json", {})
        installed = read_json(source / "plugins/installed_plugins.json", {"version": 2, "plugins": {}})
        known = read_json(source / "plugins/known_marketplaces.json", {})
        plugin_names = ["ponytail@ponytail", "caveman@caveman", "i-have-adhd@i-have-adhd"]
        for name, profile in state.get("profiles", {}).items():
            directory = root / "profiles" / name
            if directory != Path(profile["config_dir"]) or directory.is_symlink() or not directory.is_dir():
                raise ValueError(f"unexpected profile directory: {directory}")
            settings_path = directory / "settings.json"
            settings = read_json(settings_path, {})
            if not isinstance(settings, dict):
                raise ValueError(f"expected JSON object: {settings_path}")
            local_installed = read_json(directory / "plugins/installed_plugins.json", {"version": 2, "plugins": {}})
            local_known = read_json(directory / "plugins/known_marketplaces.json", {})
            for plugin in plugin_names:
                market = plugin.split("@")[1]
                settings.setdefault("enabledPlugins", {}).setdefault(plugin, defaults.get("enabledPlugins", {}).get(plugin, True))
                if market in defaults.get("extraKnownMarketplaces", {}):
                    settings.setdefault("extraKnownMarketplaces", {}).setdefault(market, defaults["extraKnownMarketplaces"][market])
                if plugin in installed["plugins"]:
                    local_installed.setdefault("plugins", {}).setdefault(plugin, installed["plugins"][plugin])
                if market in known:
                    local_known.setdefault(market, known[market])
                for folder in ("cache", "marketplaces"):
                    link_missing(source / "plugins" / folder / market, directory / "plugins" / folder / market)
            pretool = settings.setdefault("hooks", {}).setdefault("PreToolUse", [])
            rtk_hook = {"matcher": "Bash", "hooks": [{"type": "command", "command": "rtk hook claude"}]}
            if not any(h.get("command") == "rtk hook claude" for group in pretool for h in group.get("hooks", [])):
                pretool.append(rtk_hook)
            save_json(settings_path, settings)
            save_json(directory / "plugins/installed_plugins.json", local_installed)
            save_json(directory / "plugins/known_marketplaces.json", local_known)
            for filename in ("RTK.md", ".i-have-adhd-always"):
                link_missing(source / filename, directory / filename)
            instructions = directory / "CLAUDE.md"
            if instructions.is_symlink():
                raise ValueError(f"refusing to edit symlink: {instructions}")
            text = instructions.read_text() if instructions.exists() else ""
            if "@RTK.md" not in text:
                instructions.write_text("@RTK.md\n" + text)
            print(f"Defaults synced: {name}")


try:
    sync()
except (OSError, ValueError, KeyError, TypeError) as error:
    print(f"claude-defaults-sync: {error}", file=sys.stderr)
    sys.exit(1)
PY
