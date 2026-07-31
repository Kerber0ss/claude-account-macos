# claude-account

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

[Русская версия](README.ru.md)

This repository is a fork of [`hamzarehmandeveloper/claude-account`](https://github.com/hamzarehmandeveloper/claude-account/tree/main).

A macOS profile switcher for Claude Code. It gives Claude Code an isolated
`CLAUDE_CONFIG_DIR` for each account and transparently forwards normal commands
to the official Claude executable.

```bash
claude account add work
claude account add personal
claude account use work
claude account list
claude account current
claude account remove personal

claude
claude "fix this bug in main.py"
```

For OAuth profiles, Claude Code itself performs login, logout, credential
storage, and token refresh. API profiles use a separate per-profile `settings.json`
file with owner-only permissions.

> [!IMPORTANT]
> This is an independent community project. It is not made, endorsed, or
> supported by Anthropic. Claude and Claude Code are products of Anthropic.

## Requirements

- macOS on Apple Silicon
- A working Claude Code installation
- Rust 1.85 or later to build from source

## Install from source

```bash
cargo build --locked --release
./target/release/claude-account install
```

The installer prints one `export PATH=...` line. Add that line to `~/.zshrc`
and open a new terminal. The shim lives in its own directory; it does not
replace the official Claude executable.

Confirm the installation:

```bash
type -a claude
claude account list
```

The claude-account shim should appear before the official Claude executable.

## Commands

### Add an account

```bash
claude account add work
claude account add personal --email you@example.com
claude account add company --sso
claude account add api-billing --console
```

This opens Claude Code's official login flow. The first profile becomes active.
Adding another profile does not switch the active profile. The command also
completes Claude Code's local onboarding state, so the next `claude` launch
uses the saved login without asking you to authenticate again.

When no authentication option is passed, `claude account add NAME` asks whether
to use browser-based OAuth or an API configuration file. Use `--oauth` to skip
the prompt and force OAuth.

### Add an API profile

```bash
claude account add gateway --api
```

This does not open a browser. It creates
`~/Library/Application Support/claude-account/profiles/gateway/settings.json` and
prints its exact path. Fill in the generated template before starting Claude:

```json
{
  "env": {
    "ANTHROPIC_API_KEY": "your-api-key",
    "ANTHROPIC_BASE_URL": "https://gateway.example/v1",
    "ANTHROPIC_MODEL": "your-model",
    "CLAUDE_CODE_SUBAGENT_MODEL": "your-model"
  },
  "model": ""
}
```

`ANTHROPIC_API_KEY` is required; `ANTHROPIC_BASE_URL`, `ANTHROPIC_MODEL`, and
`CLAUDE_CODE_SUBAGENT_MODEL` are optional. Set the last value to the same model
to make every Claude Code subagent use your gateway model. The endpoint must be compatible with the Anthropic API used by
Claude Code; an OpenAI-only endpoint needs a compatible proxy or gateway.

### Switch accounts

```bash
claude account use work
```

Switching affects newly launched Claude processes. Existing sessions keep the
account with which they were started.

### Inspect profiles

```bash
claude account list
claude account current
```

`current` prints only the profile name, making it safe to use in scripts.

### Remove an account

```bash
claude account remove personal
```

This runs Claude Code's official `auth logout` inside the profile and
unregisters it. Settings and session history are preserved, allowing the same
profile name to reuse them later.

To delete all local data belonging to the profile:

```bash
claude account remove personal --purge --yes
```

Removing the active profile is refused unless `--force` is supplied.
`--purge` permanently deletes that profile's settings, sessions, plugins, and
history in addition to its stored login.

### Get help

```bash
claude account --help
claude account add --help
claude account remove --help
```

All non-account commands and flags are passed unchanged to the official Claude
executable:

```bash
claude
claude -p "explain this project"
claude --model opus
claude auth status --text
```

## Storage

By default:

```text
~/Library/Application Support/claude-account/state.json
~/Library/Application Support/claude-account/profiles/<name>/
~/Library/Application Support/claude-account/profiles/<name>/settings.json
~/Library/Application Support/claude-account/bin/claude
~/Library/Application Support/claude-account/libexec/claude-account
```

The optional `XDG_CONFIG_HOME` and `XDG_DATA_HOME` variables are respected.
`CLAUDE_ACCOUNT_HOME` can place all application data under one absolute
directory, which is especially useful for tests.

The state file contains profile names, authentication types, directory paths,
and the real Claude executable path. It never contains access or refresh
tokens. API keys are stored only in the selected API profile's `settings.json`,
which is created with `0600` permissions.

## Authentication environment variables

To guarantee that the selected profile is actually used, the wrapper removes
these variables from the child Claude process:

- `ANTHROPIC_API_KEY`
- `ANTHROPIC_AUTH_TOKEN`
- `CLAUDE_CODE_OAUTH_TOKEN`

Set `CLAUDE_ACCOUNT_PRESERVE_AUTH_ENV=1` if you intentionally want those
variables to override profile authentication.

For an API profile, its `settings.json` supplies `ANTHROPIC_API_KEY` and, when
configured, `ANTHROPIC_BASE_URL` and `ANTHROPIC_MODEL` to the Claude process.

## Development

```bash
cargo fmt --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the contribution workflow and
[SECURITY.md](SECURITY.md) for private vulnerability reporting.

## License

Released under the [MIT License](LICENSE).
