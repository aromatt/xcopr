#!/usr/bin/env bash

# Test: Map Mode - Word Reversal in JSON
# This test validates xcopr's map mode by reversing words within JSON messages

set -e

# Create temporary directory and ensure cleanup
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

# Create input data
cat > "$TMPDIR/input.jsonl" <<EOF
{"word":"hello","msg":"hello world"}
EOF

echo "# Input data:"
cat "$TMPDIR/input.jsonl"

echo
echo "# Command:"
CMD="xcopr map -c 'jq -r .word | rev' jq -c '.msg |= gsub(.word; \"%1\")' < $TMPDIR/input.jsonl"
echo "$CMD"

echo
echo "# Output:"
eval "$CMD"