#!/usr/bin/env bash
# Publish one workspace crate to crates.io.
#
# Same contract as npm-publish.sh: an already-published version is skipped, anything else
# fails the workflow. Asking the registry up front beats matching cargo's error text,
# which is not a stable interface.
set -euo pipefail

crate="${1:?usage: crates-publish.sh <crate-name>}"
version=$(cat VERSION)

# crates.io rejects requests without a descriptive User-Agent with 403 — which must not be
# read as "not published yet", so the status code is branched on explicitly.
status=$(curl -s --retry 3 -o /dev/null -w '%{http_code}' \
  -A "iyulab-m3l-release (https://github.com/iyulab/m3l)" \
  "https://crates.io/api/v1/crates/$crate/$version")

case "$status" in
  200)
    echo "skip: $crate $version is already on crates.io"
    exit 0
    ;;
  404) ;;
  *)
    echo "crates.io lookup for $crate $version returned HTTP $status — refusing to publish blind" >&2
    exit 1
    ;;
esac

echo "publish: $crate $version"
cargo publish -p "$crate"
