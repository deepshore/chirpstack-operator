#!/bin/sh

. ./lib.sh

export VERSION="$(git describe --tags)"
export OPERATOR_IMAGE="chirpstack-operator:${VERSION}"
export BUNDLE_IMAGE="chirpstack-operator-bundle:${VERSION}"

retry olm_install_local_registry
