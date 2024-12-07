#!/bin/sh

: "${REGISTRY:?Need to set REGISTRY}"
: "${BUNDLE_IMAGE:?Need to set BUNDLE_IMAGE}"
git describe 1>/dev/null 2>/dev/null || { echo "Not a release"; exit 1; }

TEMPLATE_FILE="catalog-template.yaml"
DOCKER_IMAGE="${REGISTRY}/${BUNDLE_IMAGE}"

if cat "${TEMPLATE_FILE}" | grep "${BUNDLE_IMAGE}" 1>/dev/null 2>/dev/null
then
  echo "Catalog up-to-date"
  exit 0
else
  yq -i '.Candidate.Bundles = .Candidate.Bundles + [{ "Image": "'"${DOCKER_IMAGE}"'" }]' \
    catalog-template.yaml
fi
