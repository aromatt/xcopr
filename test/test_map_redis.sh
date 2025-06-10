#!/usr/bin/env bash

# Test: Map Mode - Redis Bulk Queries
# This test validates xcopr's map mode with Redis bulk queries for data enrichment

set -e

# Create temporary directory and ensure cleanup
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

# Create input data
cat > "$TMPDIR/input.jsonl" <<EOF
{"name":"alice","id":1}
{"name":"billy","id":2}
EOF

echo "# Input data:"
cat "$TMPDIR/input.jsonl"

echo
echo "# Command:"
CMD="xcopr map -s 'GET id:%{jq .id}' jq -c '.birthday = \"%1{redis-cli --raw}\"' < $TMPDIR/input.jsonl"
echo "$CMD"

echo
echo "# Output:"
eval "$CMD"