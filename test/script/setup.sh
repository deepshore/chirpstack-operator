#!/bin/sh

CONTROLLER_IMAGE=chirpstack-controller:latest
BUNDLE_IMAGE=chirpstack-operator-bundle:v0.1.0
DOCKER_REGISTRY_PORT=5000
DOCKER_REGISTRY_HOST=localhost
DOCKER_REGISTRY=${DOCKER_REGISTRY_HOST}:${DOCKER_REGISTRY_PORT}

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
      minikube_running || minikube start --addons=registry --cpus=4
  } &&
  {
      olm_running || operator-sdk olm install --version v0.28.0
  } &&
  {
    REGISTRY_IP=$(kubectl -n kube-system get service registry -o jsonpath='{.spec.clusterIP}') &&
    cd config/manager && kustomize edit set image chirpstack-controller=${REGISTRY_IP}/chirpstack-controller && cd ../.. &&
    rm -fr bundle* &&
    kustomize build config/manifests | operator-sdk generate bundle -q --overwrite --version 0.1.0 &&
    operator-sdk bundle validate ./bundle &&
    {
      kubectl port-forward --namespace kube-system service/registry ${DOCKER_REGISTRY_PORT}:80 > /dev/null &
      PID="$!"
      docker buildx build --tag ${DOCKER_REGISTRY}/${CONTROLLER_IMAGE} -f Dockerfile . &&
      docker push ${DOCKER_REGISTRY}/${CONTROLLER_IMAGE} &&
      docker buildx build --tag ${DOCKER_REGISTRY}/${BUNDLE_IMAGE} -f bundle.Dockerfile . &&
      docker push ${DOCKER_REGISTRY}/${BUNDLE_IMAGE} &&
      {
        operator-sdk cleanup chirpstack-operator --namespace operators --delete-all ||
        true
      } &&
      operator-sdk run bundle --use-http ${DOCKER_REGISTRY}/${BUNDLE_IMAGE} --namespace operators --timeout 5m0s

      kill "$PID"
    }
  }
} &&
echo "Cluster+Operator running"
