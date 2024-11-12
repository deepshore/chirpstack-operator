#!/bin/sh

minikube_running()
{
  minikube status 2>/dev/null 1>/dev/null
}

start_minikube()
{
  minikube_running || minikube start --cpus="$MINIKUBE_CPUS" --memory="$MINIKUBE_MEM" $@
}

olm_running()
{
  operator-sdk olm status 2>/dev/null 1>/dev/null
}

olm_operator_running()
{
  kubectl get subscription chirpstack-operator-v0-1-0-sub -n operators 2>/dev/null 1>/dev/null &&
    test "$(kubectl get deploy chirpstack-operator-controller-manager -n operators -o yaml | yq '.status.readyReplicas')" = "1"
}

olm_remove_operator()
{
  ! olm_operator_running ||
    operator-sdk cleanup chirpstack-operator --namespace operators --delete-all
}
