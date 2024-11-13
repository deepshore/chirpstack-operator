#!/bin/sh

. $(dirname $0)/lib.sh

OPERATOR_IMAGE=chirpstack-operator:latest
BUNDLE_IMAGE=chirpstack-operator-bundle:v0.1.0
DOCKER_REGISTRY_PORT=5000
DOCKER_REGISTRY_HOST=localhost
DOCKER_REGISTRY=${DOCKER_REGISTRY_HOST}:${DOCKER_REGISTRY_PORT}
REGISTRY_IP=$(kubectl -n kube-system get service registry -o jsonpath='{.spec.clusterIP}') &&

olm_operator_running "${REGISTRY_IP}/${OPERATOR_IMAGE}" ||
{
  start_minikube &&
  minikube addons enable registry &&
  cd config/manager && kustomize edit set image chirpstack-operator=${REGISTRY_IP}/chirpstack-operator && cd ../.. &&
  rm -fr bundle* &&
  operator-sdk generate kustomize manifests --package chirpstack-operator -q &&
  kustomize build config/manifests | operator-sdk generate bundle -q --overwrite --version 0.1.0 &&
  operator-sdk bundle validate ./bundle &&
  {
    kubectl port-forward --namespace kube-system service/registry ${DOCKER_REGISTRY_PORT}:80 > /dev/null &
    PID="$!"
    docker buildx build --tag ${DOCKER_REGISTRY}/${OPERATOR_IMAGE} -f Dockerfile . &&
    docker push ${DOCKER_REGISTRY}/${OPERATOR_IMAGE} &&
    docker buildx build --tag ${DOCKER_REGISTRY}/${BUNDLE_IMAGE} -f bundle.Dockerfile . &&
    docker push ${DOCKER_REGISTRY}/${BUNDLE_IMAGE} &&
    olm_install_operator ${DOCKER_REGISTRY}/${BUNDLE_IMAGE} --use-http

    kill "$PID"
  }
} &&
echo "Cluster+Operator running"
