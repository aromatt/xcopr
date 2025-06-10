#!/usr/bin/env bash

# Test: Filter Mode - Field Equality
# This test validates xcopr's filter mode by checking JSON field equality

set -e

# Create temporary directory and ensure cleanup
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

# Create input data
cat > "$TMPDIR/input.tsv" <<EOF
alice	{"foo":0,"bar":1}
billy	{"foo":1,"bar":1}
charlie	{"bar":0,"foo":1}
EOF

echo "# Input data:"
cat "$TMPDIR/input.tsv"

echo
echo "# Command:"
CMD="xcopr f -c 'cut -f2 | jq \".foo == .bar\"' -e true < $TMPDIR/input.tsv"
echo "$CMD"

echo
echo "# Output:"
eval "$CMD"