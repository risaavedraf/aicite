#!/bin/bash
# Re-ingest Cite self-documentation
# Run from project root: bash scripts/ingest-self-docs.sh
#
# This script deletes the existing corpus and re-ingests all project documentation.
# Use after significant documentation changes to keep the corpus fresh.

set -e

CITE="${CITE_BIN:-./target/release/cite}"
CWD="$(pwd)"
DATA_DIR="${CITE_DATA_DIR:-$HOME/AppData/Roaming/cite}"

echo "=== Cite Self-Documentation Ingestion ==="
echo "Binary: $CITE"
echo "Data dir: $DATA_DIR"
echo ""

# Verify binary exists
if [ ! -f "$CITE" ]; then
	echo "ERROR: Binary not found at $CITE"
	echo "Run 'cargo build --release' first."
	exit 1
fi

# Step 1: Clean existing corpus
echo "--- Cleaning existing corpus ---"
rm -f "$DATA_DIR/cite.db" "$DATA_DIR/cite.db-wal" "$DATA_DIR/cite.db-shm"
echo "Database deleted."

# Step 2: Ingest documents
OK=0
FAIL=0

ingest() {
	local file="$1"
	local display_name="${2:-$(basename "$file")}"

	if [ ! -f "$file" ]; then
		echo "  SKIP: $file"
		return
	fi

	if $CITE ingest "$file" --display-name "$display_name" --no-banner 2>&1; then
		OK=$((OK + 1))
	else
		echo "  FAIL: $file"
		FAIL=$((FAIL + 1))
	fi
}

echo ""
echo "--- Ingesting root documentation ---"
ingest "README.md" "README.md"
ingest "CHANGELOG.md" "CHANGELOG.md"
ingest "CONTRIBUTING.md" "CONTRIBUTING.md"
ingest "SECURITY.md" "SECURITY.md"
ingest "CODE_OF_CONDUCT.md" "CODE_OF_CONDUCT.md"

echo ""
echo "--- Ingesting OpenSpec index ---"
ingest "openspec/index.md" "OpenSpec Index"

echo ""
echo "--- Ingesting architecture docs ---"
for f in openspec/architecture/*.md; do
	ingest "$f" "$(basename "$f" .md)"
done

echo ""
echo "--- Ingesting guides ---"
for f in openspec/guides/*.md; do
	ingest "$f" "$(basename "$f" .md)"
done

echo ""
echo "--- Ingesting PRD ---"
for f in openspec/prd/*.md; do
	ingest "$f" "$(basename "$f" .md)"
done

echo ""
echo "--- Ingesting improvements & ideas ---"
for f in openspec/improvements/ideas/*.md; do
	ingest "$f" "$(basename "$f" .md)"
done

echo ""
echo "--- Ingesting RFCs ---"
find openspec/rfc -name "*.md" | while read f; do
	ingest "$f" "$(basename "$f" .md)"
done

echo ""
echo "--- Ingesting specs ---"
find openspec/specs -name "*.md" | while read f; do
	ingest "$f" "$(basename "$f" .md)"
done

echo ""
echo "=== Ingestion complete ==="
echo "  OK:   $OK"
echo "  FAIL: $FAIL"
echo ""

# Show final state
echo "--- Final corpus state ---"
$CITE health --json --no-banner
