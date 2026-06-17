#!/usr/bin/env bash
# Install git hooks for wiggum development

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GIT_DIR="$(git rev-parse --git-dir)"

echo "Installing git hooks..."

# Install pre-commit hook
cp "$SCRIPT_DIR/pre-commit" "$GIT_DIR/hooks/pre-commit"
chmod +x "$GIT_DIR/hooks/pre-commit"

echo "✓ Pre-commit hook installed"
echo ""
echo "The hook will:"
echo "  • Run cargo fmt and auto-stage formatting changes"
echo "  • Run gitleaks to detect secrets and credentials"
echo "  • Run cargo audit to check for security vulnerabilities"
echo ""
echo "To bypass the hook (not recommended), use: git commit --no-verify"
