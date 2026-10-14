#!/usr/bin/env bash
# Runs the plugin's mini.test suite under `nvim --headless`.
# Fetches mini.nvim into nvim/tests/deps/ (gitignored) on first run.
set -euo pipefail

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mini_dir="$tests_dir/deps/mini.nvim"

if [ ! -d "$mini_dir" ]; then
  mkdir -p "$mini_dir"
  curl -fsSL https://github.com/echasnovski/mini.nvim/archive/refs/heads/main.tar.gz |
    tar -xz -C "$mini_dir" --strip-components=1
fi

cd "$tests_dir/.."
# The stdout reporter quits with exit 1 on any failure; the trailing cquit
# covers errors raised before the reporter runs.
nvim --headless --noplugin -i NONE -u tests/minimal_init.lua \
  -c "lua MiniTest.run({ execute = { reporter = MiniTest.gen_reporter.stdout({ group_depth = 1 }) } })" \
  -c 'cquit 1'
