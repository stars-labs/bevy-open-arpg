#!/usr/bin/env bash

set -euo pipefail

WORKFLOW="deploy-wasm-pages.yml"
RELEASE_TAG_RE='^v[0-9]+\.[0-9]+\.[0-9]+$'

current_ref() {
  local ref
  ref="$(git branch --show-current || true)"
  if [ -n "$ref" ]; then
    printf '%s\n' "$ref"
  else
    git rev-parse HEAD
  fi
}

ensure_remote_tag() {
  local tag=$1
  if ! git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1; then
    echo "Pushing tag $tag to origin..."
    git push origin "$tag"
  fi
}

require_gh() {
  if ! command -v gh >/dev/null 2>&1; then
    echo "Error: GitHub CLI is required. Install from https://cli.github.com/ and run 'gh auth login' first." >&2
    exit 10
  fi
}

require_clean_tree_for_tag_creation() {
  if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "Refusing to create a tag while the working tree has uncommitted changes." >&2
    echo "Commit or stash your changes first, or run with an existing tag." >&2
    exit 11
  fi
}

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/release_github.sh preview
    Trigger the rolling web-only preview release (`web-latest`) from current ref.

  ./scripts/release_github.sh preview <ref>
    Trigger the rolling web-only preview release (`web-latest`) from a branch/tag/commit.

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
    CURRENT_REF="${2:-$(current_ref)}"
    echo "Triggering GitHub preview release (web-latest) from ${CURRENT_REF}..."
    gh workflow run "$WORKFLOW" \
      --ref "$CURRENT_REF" \
      -f checkout_ref="$CURRENT_REF" \
      -f release_type=preview
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
      require_clean_tree_for_tag_creation
      if ! git rev-parse "$TAG" >/dev/null 2>&1; then
        echo "Creating tag $TAG..."
        git tag -a "$TAG" -m "Release $TAG"
        git push origin "$TAG"
      else
        echo "Tag $TAG already exists locally; continuing without re-creating."
      fi
      ensure_remote_tag "$TAG"
    else
      TAG="$2"
      validate_tag "$TAG"
      ensure_remote_tag "$TAG"
    fi

    if ! git show-ref --verify --quiet "refs/tags/$TAG"; then
      echo "Fetching tag $TAG from origin..."
      git fetch origin "refs/tags/$TAG:refs/tags/$TAG"
    fi

    echo "Triggering GitHub versioned release for $TAG..."
    gh workflow run "$WORKFLOW" \
      --ref "$TAG" \
      -f release_type=tag \
      -f release_tag="$TAG"
    ;;

  *)
    echo "Unknown mode: $1" >&2
    usage
    exit 1
    ;;
esac
