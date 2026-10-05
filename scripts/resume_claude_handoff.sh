#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec claude --resume deda028f-517c-4709-804b-3a8067d8ee8b --tools default --system-prompt-snapshot off "$@"
