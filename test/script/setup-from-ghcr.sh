#!/bin/sh

OPERATOR_IMAGE=chirpstack-operator:latest
BUNDLE_IMAGE=chirpstack-operator-bundle:v0.1.0
REGISTRY=ghcr.io/deepshore

operator_running()
{
  kubectl get subscription chirpstack-operator-v0-1-0-sub -n operators 2>/dev/null 1>/dev/null &&
    test "$(kubectl get deploy chirpstack-operator-controller-manager -n operators -o yaml | yq '.status.readyReplicas')" = "1"
}

minikube_running()
{
  minikube status 2>/dev/null 1>/dev/null
}

olm_running()
{
  operator-sdk olm status 2>/dev/null 1>/dev/null
}

operator_running ||
{
  {
    minikube_running || minikube start --cpus=4
  } &&
  {
    olm_running || operator-sdk olm install --version v0.28.0
  } &&
  {
    operator-sdk cleanup chirpstack-operator --namespace operators --delete-all || true
  } &&
  {
      operator-sdk run bundle ${REGISTRY}/${BUNDLE_IMAGE} --namespace operators --timeout 5m0s
  }
} &&
echo "Cluster+Operator running"
