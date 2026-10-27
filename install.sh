#!/bin/sh
set -eu

# Installs from source; Cargo, Git, and a C compiler are required.
repo="https://github.com/danew/enver.git"
skill_dir="${ENVCTL_SKILL_DIR:-${HOME}/.agents/skills/envctl}"
if [ -e "$skill_dir" ] || [ -L "$skill_dir" ]; then
  printf 'Skill already exists at %s; remove or relocate it before installing.\n' "$skill_dir" >&2
  exit 1
fi

temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/enver-install.XXXXXXXX")
trap 'rm -rf "$temp_dir"' EXIT HUP INT TERM
git clone --depth 1 "$repo" "$temp_dir/enver"
cargo install --path "$temp_dir/enver" --locked --force
mkdir -p "$(dirname "$skill_dir")"
mkdir "$skill_dir"
cp "$temp_dir/enver/.agents/skills/envctl/SKILL.md" "$skill_dir/SKILL.md"
printf 'Installed envctl and agent skill at %s\n' "$skill_dir"
