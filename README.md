# enver

An experimental Rust CLI for inspecting and updating `.env` files with visibility-aware JSON output. The executable is named `enver`.

Use it to give an agent configuration metadata without returning values marked private. Public values are returned in full; private values return only existence, inferred type, and format.

> **Security warning:** URL/URI-shaped values are **public by default**, including URLs containing credentials or tokens. Configure private key patterns or explicit private sidecar entries before using `get` or `list` on files containing sensitive URLs. This tool is not an access-control boundary: a process with permission to read the `.env` file can bypass it.

## Install

**Ask your agent:**

> Install https://github.com/danew/enver, including the `enver` CLI and its agent skill, following the repository's installation instructions.

Or install both yourself (requires Git, Rust/Cargo, and a C compiler):

```sh
curl -fsSL https://raw.githubusercontent.com/danew/enver/main/install.sh -o enver-install.sh
sh enver-install.sh
```

Review the script before running it if desired. It installs `enver` using Cargo (replacing an existing Cargo-installed `enver`) and the skill into `~/.agents/skills/enver`. Set `ENVER_SKILL_DIR` to a different skill directory if your agent uses another location. The installer refuses to overwrite an existing skill; remove or relocate it before upgrading. Ensure Cargo's bin directory is on your `PATH`. Your agent may need to restart to discover a newly installed skill.

To install just the CLI from a local clone:

```sh
git clone git@github.com:danew/enver.git
cd enver
cargo install --path . --locked
```

Ensure Cargo's bin directory is on your `PATH`. Alternatively run directly with `cargo run --manifest-path /path/to/enver/Cargo.toml -- <command>`.

Prebuilt Linux and macOS binaries for x86-64 and ARM64 are available from [GitHub Releases](https://github.com/danew/enver/releases). Each successful push to `main` publishes a release tagged `v0.1.0-sha.<full-commit-sha>`; the tag identifies the commit, not a compatibility guarantee. Releases contain binaries only; use the installer above to install the skill too.

## Usage

Commands use `.env` in the current directory by default. Use `--file` to select another existing dotenv file.

```sh
enver ls
enver get API_KEY
enver inspect API_KEY
enver list
enver --file path/to/.env get API_URL
enver set API_URL https://example.com --visibility public
enver set FEATURE_ENABLED true --visibility private
```

`enver ls` discovers `.env` and `.env.*` files in the current directory without reading their contents. It returns a JSON array of filenames, suggested aliases, and whether each file could be opened for reading. It excludes visibility sidecars, directories, and broken symlinks. `.env` is suggested as `root`, while `.env.staging` is suggested as `staging`; `.env.root` has no alias to avoid a collision. These are labels only: continue to use the full path with `--file`. `ls` does not check whether the contents can be parsed. `enver list` remains the command for listing keys in one file.

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

The `envctl` config directory is retained for compatibility with earlier installations. Existing visibility sidecar files also continue to work.

```toml
private_key_patterns = ["TOKEN", "SECRET", "PASSWORD", "DATABASE_URL"]
public_key_patterns = ["^PUBLIC_"]
public_value_patterns = ["^https://localhost"]
```

Patterns use Rust regex syntax and match substrings unless anchored. **Invalid patterns are silently ignored**, including private patterns; check them carefully. Explicit sidecar entries override even private regexes. A nonexistent config file is treated as empty.

## Security and current limitations

- Protect `.env` using filesystem permissions. `enver` must be able to read it and does not circumvent permissions. Private visibility affects CLI output only.
- Values passed to `set` are command-line arguments and may appear in shell history or process listings. There is currently no stdin or interactive secret input.
- The sidecar contains variable names and visibility, not values. Protect it from unauthorized changes: a public entry can expose a secret.
- The parser supports basic line-oriented `KEY=value` files and simple surrounding quotes. It does not fully implement dotenv quoting, escapes, inline comments, multiline values, or `export` syntax. Back up files and check compatibility before editing complex dotenv files.
- Writes to the dotenv file and sidecar are separate and non-atomic. A failure can leave the value updated without the intended visibility. Avoid concurrent writes.
- There is no sync command or separate secret store; `set` writes directly to the dotenv file.

An agent skill is included at `.agents/skills/enver/SKILL.md`; install or copy it into the location used by your agent harness.

## Development

```sh
cargo test --locked
cargo fmt --check
```
