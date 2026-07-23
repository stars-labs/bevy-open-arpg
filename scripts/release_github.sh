#!/usr/bin/env bash

set -euo pipefail

WORKFLOW="deploy-wasm-pages.yml"
RELEASE_TAG_RE='^v[0-9]+\.[0-9]+\.[0-9]+$'

require_gh() {
  if ! command -v gh >/dev/null 2>&1; then
    echo "Error: GitHub CLI is required. Install from https://cli.github.com/ and run 'gh auth login' first." >&2
    exit 10
  fi
}

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/release_github.sh preview
    Trigger the rolling web-only preview release (`web-latest`).

  ./scripts/release_github.sh tag <vX.Y.Z>
    Trigger a versioned release and native artifact build for the provided tag.

  ./scripts/release_github.sh tag --push <vX.Y.Z>
    Create the git tag first and push it, then trigger the same versioned release.

  ./scripts/release_github.sh --help
    Show this help text.
USAGE
}

validate_tag() {
  local tag=$1
  if [[ ! "$tag" =~ $RELEASE_TAG_RE ]]; then
    echo "Error: tag must match vX.Y.Z (for example v0.1.0)." >&2
    exit 3
  fi
}

if [[ $# -lt 1 || "$1" == "--help" ]]; then
  usage
  exit 1
fi

require_gh

case "$1" in
  preview)
    echo "Triggering GitHub preview release (web-latest)..."
    gh workflow run "$WORKFLOW" -f release_type=preview
    ;;

  tag)
    if [[ $# -lt 2 ]]; then
      echo "Error: tag mode requires a version (vX.Y.Z)." >&2
      usage
      exit 2
    fi

    if [[ "$2" == "--push" ]]; then
      if [[ $# -lt 3 ]]; then
        echo "Error: '--push' requires a tag argument." >&2
        usage
        exit 2
      fi
      TAG="$3"
      validate_tag "$TAG"
      if ! git rev-parse "$TAG" >/dev/null 2>&1; then
        echo "Creating tag $TAG..."
        git tag -a "$TAG" -m "Release $TAG"
        git push origin "$TAG"
      else
        echo "Tag $TAG already exists locally; continuing without re-creating."
      fi
    else
      TAG="$2"
      validate_tag "$TAG"
    fi

    echo "Triggering GitHub versioned release for $TAG..."
    gh workflow run "$WORKFLOW" -f release_type=tag -f release_tag="$TAG"
    ;;

  *)
    echo "Unknown mode: $1" >&2
    usage
    exit 1
    ;;
esac
