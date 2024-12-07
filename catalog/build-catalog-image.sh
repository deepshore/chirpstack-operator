#!/bin/sh

: "${REGISTRY:?Need to set REGISTRY}"
: "${BUNDLE_IMAGE:?Need to set BUNDLE_IMAGE}"
git describe 1>/dev/null 2>/dev/null || { echo "Not a release"; exit 1; }

CATALOG_DIR="chirpstack-operator-catalog"
CATALOG_FILE="catalog.yaml"
TEMPLATE_FILE="catalog-template.yaml"
DOCKER_IMAGE="${REGISTRY}/${BUNDLE_IMAGE}"

wget https://github.com/operator-framework/operator-registry/releases/download/v1.48.0/linux-amd64-opm &&
chmod +x linux-amd64-opm &&
mkdir -p "${CATALOG_DIR}" &&
./linux-amd64-opm alpha render-template semver -o yaml < "${TEMPLATE_FILE}" > "${CATALOG_DIR}/${CATALOG_FILE}" &&
{ ./linux-amd64-opm validate "${CATALOG_DIR}" || { echo "Validation failed"; exit 1; }; } &&
./linux-amd64-opm generate dockerfile "${CATALOG_DIR}" &&
docker push ${DOCKER_IMAGE}
