#!/bin/sh

: "${REGISTRY:?Need to set REGISTRY}"
git describe 1>/dev/null 2>/dev/null || { echo "Not a release"; exit 1; }

CATALOG_NAME="chirpstack-operator-catalog"
CATALOG_FILE="catalog.yaml"
TEMPLATE_FILE="catalog-template.yaml"
DOCKER_IMAGE="${REGISTRY}/${CATALOG_NAME}:latest"
rm -fr "${CATALOG_NAME}"
rm "${CATALOG_NAME}.Dockerfile"

{
  test ./linux-amd64-opm ||
  wget https://github.com/operator-framework/operator-registry/releases/download/v1.48.0/linux-amd64-opm
} &&
chmod +x linux-amd64-opm &&
mkdir -p "${CATALOG_NAME}" &&
./linux-amd64-opm alpha render-template semver -o yaml < "${TEMPLATE_FILE}" > "${CATALOG_NAME}/${CATALOG_FILE}" &&
{ ./linux-amd64-opm validate "${CATALOG_NAME}" || { echo "Validation failed"; exit 1; }; } &&
./linux-amd64-opm generate dockerfile "${CATALOG_NAME}" &&
docker build . -f ${CATALOG_NAME}.Dockerfile -t ${DOCKER_IMAGE} &&
docker push ${DOCKER_IMAGE}
