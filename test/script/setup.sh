#!/bin/sh

CONTROLLER_IMAGE=chirpstack-controller:latest
BUNDLE_IMAGE=chirpstack-operator-bundle:v0.1.0
DOCKER_REGISTRY=localhost:5000

{ minikube status 2>/dev/null 1>/dev/null || minikube start --addons=registry; } &&
{ operator-sdk olm status 2>/dev/null 1>/dev/null || operator-sdk olm install --version v0.28.0; } &&
kubectl apply -k test/dep &&
REGISTRY_IP=$(kubectl -n kube-system get service registry -o jsonpath='{.spec.clusterIP}') &&
cd config/manager && kustomize edit set image chirpstack-controller=${REGISTRY_IP}/chirpstack-controller && cd ../.. &&
rm -fr bundle* &&
operator-sdk generate kustomize manifests --package chirpstack-operator -q &&
kustomize build config/manifests | operator-sdk generate bundle -q --overwrite --version 0.1.0 &&
operator-sdk bundle validate ./bundle &&
{
  kubectl port-forward --namespace kube-system service/registry 5000:80 > /dev/null &
  PID="$!"
  docker buildx build --tag ${DOCKER_REGISTRY}/${CONTROLLER_IMAGE} -f Dockerfile . &&
  docker push ${DOCKER_REGISTRY}/${CONTROLLER_IMAGE} &&
  docker buildx build --tag ${DOCKER_REGISTRY}/${BUNDLE_IMAGE} -f bundle.Dockerfile . &&
  docker push ${DOCKER_REGISTRY}/${BUNDLE_IMAGE} &&
  {
    operator-sdk cleanup chirpstack-operator --namespace chirpstack --delete-all ||
    true
  } &&
  operator-sdk run bundle --use-http ${DOCKER_REGISTRY}/${BUNDLE_IMAGE} --namespace chirpstack --timeout 5m0s

  kill "$PID"
}
