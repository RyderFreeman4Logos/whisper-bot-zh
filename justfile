default:
    @just --list

rust_manifest := "rust/Cargo.toml"

fmt:
    cargo fmt --manifest-path {{rust_manifest}} --all

fmt-check:
    cargo fmt --manifest-path {{rust_manifest}} --all -- --check

clippy:
    cargo clippy --manifest-path {{rust_manifest}} --workspace --all-targets -- -D warnings

build:
    cargo build --manifest-path {{rust_manifest}} --workspace

test:
    cargo test --manifest-path {{rust_manifest}} --workspace

find-monolith-files:
    #!/usr/bin/env bash
    set -euo pipefail
    THRESHOLD_TOKENS="${MONOLITH_TOKEN_THRESHOLD:-8000}"
    MODEL="${TOKUIN_MODEL:-gpt-4}"

    _monolith_error() {
        local file="$1" actual="$2" limit="$3"
        echo ""
        echo "=========================================="
        echo "ERROR: Monolith file detected! ($actual, limit: $limit)"
        echo "  File: $file"
        echo "=========================================="
        echo ""
        echo "REQUIRED ACTION:"
        echo "1. Stash your current work first:  git stash push -m 'pre-split'"
        echo "2. Split this file:                /split-monolith-files"
        echo "3. After splitting, retry your commit."
        echo ""
        echo "WHY: Large files cause context window bloat and degrade LLM performance."
        echo "=========================================="
    }

    check_file() {
        local file="$1"
        local threshold_tokens="$2"
        local model="$3"
        case "$file" in
            *.lock|*lock.json|*lock.yaml) return 0 ;;
            *.min.*) return 0 ;;
            */AGENTS.md|*/FACTORY.md) return 0 ;;
            *.md) return 0 ;;
            *.toml|.env*|LICENSE|.python-version) return 0 ;;
            uv.lock|Cargo.lock|package-lock.json|pnpm-lock.yaml|weave.lock) return 0 ;;
        esac
        [ -f "$file" ] || return 0
        grep -Iq '' "$file" 2>/dev/null || return 0

        local tokens
        tokens=$(tokuin estimate --model "$model" --format json "$file" 2>/dev/null \
            | jq -r '.tokens // 0')
        tokens="${tokens:-0}"
        if [ "$tokens" -gt "$threshold_tokens" ]; then
            _monolith_error "$file" "$tokens tokens" "$threshold_tokens tokens"
            return 1
        fi
        return 0
    }
    export -f check_file _monolith_error

    git ls-files --recurse-submodules \
        | parallel --halt now,fail=1 check_file {} "$THRESHOLD_TOKENS" "$MODEL"

pre-commit: find-monolith-files fmt-check clippy test
