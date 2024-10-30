CONTROLLER_IMAGE := chirpstack-controller:latest
BUNDLE_IMAGE := chirpstack-operator-bundle:v0.1.0

default: build manifest

image:
	docker buildx build --tag $(CONTROLLER_IMAGE) -f Dockerfile . &&

config/crd/bases/applications.deepshore.de_chirpstacks.yaml: chirpstack-operator/src/crd.rs
	cargo build
	cargo run --bin make-crd-manifest | yq -o yaml -P > $@ || rm -f $@

build:
	cargo build

run-controller: install
	cargo build
	RUST_LOG=debug cargo run --bin controller

install: config/crd/bases/applications.deepshore.de_chirpstacks.yaml
	kubectl apply -k config/crd

uninstall:
	kubectl delete -k config/crd

uninstall:
	kubectl delete -k config/crd

deploy-sample:
	kubectl apply -k test/sample

undeploy-sample:
	kubectl delete -k test/sample

bundle:
	operator-sdk generate kustomize manifests --package chirpstack-operator -q
	kustomize build config/manifests | operator-sdk generate bundle -q --overwrite --version 0.1.0
	operator-sdk bundle validate ./bundle

bundle-image: bundle
	docker buildx build --tag $(BUNDLE_IMAGE) -f bundle.Dockerfile . &&

minikube: config/crd/bases/applications.deepshore.de_chirpstacks.yaml
	minikube status 2>/dev/null 1>/dev/null || minikube start --addons=registry
	kubectl apply -k test/dep
	kubectl apply -k config/crd

test-cluster:
	sh test/script/setup.sh

clean:
	rm -fr bundle*
	minikube delete
