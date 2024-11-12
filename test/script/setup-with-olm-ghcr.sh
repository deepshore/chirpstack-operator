#!/bin/sh

. $(dirname $0)/lib.sh

OPERATOR_IMAGE=chirpstack-operator:latest
BUNDLE_IMAGE=chirpstack-operator-bundle:v0.1.0
REGISTRY=ghcr.io/deepshore

olm_install_operator ${REGISTRY}/${BUNDLE_IMAGE} &&
echo "Cluster+Operator running"
