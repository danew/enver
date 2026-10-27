# enver

An experimental Rust CLI for inspecting and updating `.env` files with visibility-aware JSON output. The executable is named `envctl`.

Use it to give an agent configuration metadata without returning values marked private. Public values are returned in full; private values return only existence, inferred type, and format.

> **Security warning:** URL/URI-shaped values are **public by default**, including URLs containing credentials or tokens. Configure private key patterns or explicit private sidecar entries before using `get` or `list` on files containing sensitive URLs. This tool is not an access-control boundary: a process with permission to read the `.env` file can bypass it.

## Install

With Rust and Cargo installed:

```sh
git clone git@github.com:danew/enver.git
cd enver
cargo install --path . --locked
```

Ensure Cargo's bin directory is on your `PATH`. Alternatively run directly with `cargo run --manifest-path /path/to/enver/Cargo.toml -- <command>`.

## Usage

Commands use `.env` in the current directory by default. Use `--file` to select another existing dotenv file.

```sh
envctl get API_KEY
envctl inspect API_KEY
envctl list
envctl --file path/to/.env get API_URL
envctl set API_URL https://example.com --visibility public
envctl set FEATURE_ENABLED true --visibility private
```

`get` and `inspect` currently produce the same JSON. For example, a private comma-separated value yields:

```json
{
  "exists": true,
  "type": "string",
  "format": "comma-separated",
  "visibility": "private"
}
```

Public values also include a `value` field. Missing keys report `exists: false`. Types and formats are heuristics, not validation; for example, `json-array` does not guarantee valid JSON. `set` defaults to private if `--visibility` is omitted. The dotenv file must already exist.

### Visibility policy

In descending order of precedence:

1. A per-key entry in the sidecar file `<dotenv-file>.visibility.json`.
2. A matching global private-key regex.
3. A matching global public-key or public-value regex.
4. Values parsed as URLs/URIs are public.
5. Everything else is private.

`set` stores its explicit visibility in the sidecar. You can also create that file manually before inspection, for example:

```json
{
  "DATABASE_URL": "private",
  "API_URL": "public"
}
```

Global settings are read from the OS config directory (typically `~/.config/envctl/config.toml` on Linux). Override the path with `--config PATH`:

```toml
private_key_patterns = ["TOKEN", "SECRET", "PASSWORD", "DATABASE_URL"]
public_key_patterns = ["^PUBLIC_"]
public_value_patterns = ["^https://localhost"]
```

Patterns use Rust regex syntax and match substrings unless anchored. **Invalid patterns are silently ignored**, including private patterns; check them carefully. Explicit sidecar entries override even private regexes. A nonexistent config file is treated as empty.

## Security and current limitations

- Protect `.env` using filesystem permissions. `envctl` must be able to read it and does not circumvent permissions. Private visibility affects CLI output only.
- Values passed to `set` are command-line arguments and may appear in shell history or process listings. There is currently no stdin or interactive secret input.
- The sidecar contains variable names and visibility, not values. Protect it from unauthorized changes: a public entry can expose a secret.
- The parser supports basic line-oriented `KEY=value` files and simple surrounding quotes. It does not fully implement dotenv quoting, escapes, inline comments, multiline values, or `export` syntax. Back up files and check compatibility before editing complex dotenv files.
- Writes to the dotenv file and sidecar are separate and non-atomic. A failure can leave the value updated without the intended visibility. Avoid concurrent writes.
- There is no sync command or separate secret store; `set` writes directly to the dotenv file.

An agent skill is included at `.agents/skills/envctl/SKILL.md`; install or copy it into the location used by your agent harness.

## Development

```sh
cargo test --locked
cargo fmt --check
```
