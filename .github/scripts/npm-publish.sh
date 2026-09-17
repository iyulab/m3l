#!/usr/bin/env bash
# Publish one npm package directory.
#
# Tolerates exactly one condition: that version is already on the registry (re-runs, and
# the platform packages a partially-failed run already pushed). Every other failure — a
# network fault, an auth fault, a rejected publish — fails the workflow. A publish step
# that swallows errors turns "the workflow was green" into a statement about nothing.
set -euo pipefail

dir="${1:?usage: npm-publish.sh <package-dir>}"
manifest="$dir/package.json"

name=$(node -p "require('./$manifest').name")
version=$(node -p "require('./$manifest').version")

if view_out=$(npm view "$name@$version" version 2>&1); then
  echo "skip: $name@$version is already on the registry"
  exit 0
fi

# Only a 404 means "not published yet". Anything else is a lookup failure, and publishing
# on top of one would be guessing.
case "$view_out" in
  *E404*) ;;
  *)
    echo "$view_out" >&2
    echo "registry lookup for $name@$version failed — refusing to publish blind" >&2
    exit 1
    ;;
esac

echo "publish: $name@$version"
cd "$dir"
npm publish --access public
