#!/bin/sh

. ./lib.sh

REGISTRY_IP=$(kubectl -n kube-system get service registry -o jsonpath='{.spec.clusterIP}') &&
OPERATOR_IMAGE=chirpstack-operator:latest &&
export BLACKJACK_OPERATOR_IMAGE="${REGISTRY_IP}/${OPERATOR_IMAGE}" &&
olm_install_local_registry
