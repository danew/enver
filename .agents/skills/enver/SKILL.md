---
name: enver
description: Inspect and update dotenv files through the enver Rust CLI without printing values classified as private. Use when working with .env files, environment configuration, variable visibility, agent-readable configuration metadata, or enver commands.
---

# enver

Use `enver` instead of reading `.env` files directly. The CLI emits JSON and does not print values classified as private. URI-like secrets can be classified public by default; see the safety notes before invoking it.

## Install

From the enver repository root:

```bash
cargo install --path . --locked
```

For a one-off invocation without installing:

```bash
cargo run --manifest-path /path/to/enver/Cargo.toml -- <command>
```

The installed executable is `enver`.

## Read configuration

```bash
enver get KEY
enver inspect KEY
enver list
```

Use `--file PATH` for a dotenv file other than `.env`:

```bash
enver --file path/to/.env list
```

Output includes `exists`, `type`, `format`, and `visibility`. The `value` field is present only for public variables. A private value must never be recovered by using another command, printing the file, or changing its visibility merely to inspect it.

## Update configuration

Set visibility explicitly when adding or changing a value:

```bash
enver set API_BASE_URL https://api.example.com --visibility public
enver set API_TOKEN '<secret>' --visibility private
```

`set` updates the dotenv file and records the explicit decision in `<dotenv-file>.visibility.json`. Do not put secrets in shell history or command-line arguments when that is a concern; prefer a secure operator workflow until stdin support is added.

## Visibility rules

Precedence is:

1. Explicit entry in `.env.visibility.json`.
2. Matching `private_key_patterns`.
3. Matching `public_key_patterns` or `public_value_patterns`.
4. URL/URI auto-detection.
5. Private by default.

Global regex configuration is read from `~/.config/envctl/config.toml`:

The config directory retains the previous CLI name for compatibility.

```toml
public_key_patterns = ["^PUBLIC_", "_URL$"]
public_value_patterns = ["^https://localhost"]
private_key_patterns = ["TOKEN", "SECRET", "PASSWORD"]
```

Use `--config PATH` to select another config file. Keep private patterns conservative; an invalid regex is ignored by the current CLI, so validate configuration changes with `enver list`.

## Safety and limitations

- Do not use `cat`, `grep`, `sed`, or equivalent commands against protected `.env` files.
- `enver` respects filesystem permissions; it does not bypass a `0600` file.
- Keep both `.env` and its visibility sidecar protected. The sidecar contains names and visibility policy, not secret values.
- URLs can contain passwords, tokens, or other secrets and are public by default. Before running `get` or `list`, configure private key patterns or explicit sidecar entries for sensitive URLs; never rely on URI detection to protect secrets.
- Run `enver list` before build/deploy validation to confirm required keys exist and have the expected metadata.
