#!/bin/sh

minikube_running()
{
  minikube status 2>/dev/null 1>/dev/null
}

olm_running()
{
  operator-sdk olm status 2>/dev/null 1>/dev/null
}

olm_any_operator_running()
{
  kubectl get subscription chirpstack-operator-v0-1-0-sub -n operators 2>/dev/null 1>/dev/null &&
    test "$(kubectl get deploy chirpstack-operator-controller-manager -n operators -o yaml | yq '.status.readyReplicas')" = "1"
}

olm_operator_running()
{
  olm_any_operator_running &&
  test "$(kubectl get deploy chirpstack-operator-controller-manager -n operators -o yaml | yq '.spec.template.spec.containers[0].image')" = "$1"
}

olm_remove_operator()
{
  ! olm_any_operator_running ||
    operator-sdk cleanup chirpstack-operator --namespace operators --delete-all
}

olm_install_olm()
{
  olm_running || operator-sdk olm install --version v0.28.0
}

start_minikube()
{
  minikube_running || minikube start --cpus="$MINIKUBE_CPUS" --memory="$MINIKUBE_MEM" "$@"
}

olm_install_operator()
{
  olm_operator_running "$1" || {
    start_minikube &&
    olm_install_olm &&
    { operator-sdk cleanup chirpstack-operator --namespace operators --delete-all 1>/dev/null 2>/dev/null || true; } &&
    operator-sdk run bundle --namespace operators --timeout 5m0s "$@"
  }
}

olm_prepare_install_local_registry()
{
  start_minikube &&
  minikube addons enable registry &&
  OPERATOR_IMAGE=chirpstack-operator:latest &&
  BUNDLE_IMAGE=chirpstack-operator-bundle:v0.1.0 &&
  DOCKER_REGISTRY_PORT=5000 &&
  DOCKER_REGISTRY_HOST=localhost &&
  DOCKER_REGISTRY=${DOCKER_REGISTRY_HOST}:${DOCKER_REGISTRY_PORT} &&
  REGISTRY_IP=$(kubectl -n kube-system get service registry -o jsonpath='{.spec.clusterIP}') &&

  {
    olm_operator_running "${REGISTRY_IP}/${OPERATOR_IMAGE}" ||
    {
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
        docker push ${DOCKER_REGISTRY}/${BUNDLE_IMAGE}
        exit_code="$?"

        kill "$PID"
        test "$exit_code" = "0"
      }
    }
  }
}

olm_install_local_registry()
{
  OPERATOR_IMAGE=chirpstack-operator:latest &&
  BUNDLE_IMAGE=chirpstack-operator-bundle:v0.1.0 &&
  DOCKER_REGISTRY_PORT=5000 &&
  DOCKER_REGISTRY_HOST=localhost &&
  DOCKER_REGISTRY=${DOCKER_REGISTRY_HOST}:${DOCKER_REGISTRY_PORT} &&
  REGISTRY_IP=$(kubectl -n kube-system get service registry -o jsonpath='{.spec.clusterIP}') &&

  {
    olm_operator_running "${REGISTRY_IP}/${OPERATOR_IMAGE}" ||
    {
      {
        operator-sdk cleanup chirpstack-operator --namespace operators --delete-all 1>/dev/null 2>/dev/null ||
        true
      } &&
      kubectl port-forward --namespace kube-system service/registry ${DOCKER_REGISTRY_PORT}:80 > /dev/null &
      PID="$!"
      olm_install_operator ${DOCKER_REGISTRY}/${BUNDLE_IMAGE} --use-http
      exit_code="$?"
      kill "$PID"
      test "$exit_code" = "0"
    }
  }
}
