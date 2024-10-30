CONTROLLER_IMAGE := chirpstack-controller:latest
BUNDLE_IMAGE := chirpstack-operator-bundle:v0.1.0

default: build manifest

image:
	docker buildx build --tag $(CONTROLLER_IMAGE) -f Dockerfile . &&

config/crd/bases/applications.deepshore.de_chirpstacks.yaml: src/crd.rs
	cargo build
	cargo run --bin make-crd-manifest | yq -o yaml -P > $@ || rm -f $@

build:
	cargo build

run-controller-local: install
	cargo build
	RUST_LOG=debug cargo run --bin controller

install: config/crd/bases/applications.deepshore.de_chirpstacks.yaml
	kubectl apply -f config/crd/bases/applications.deepshore.de_chirpstacks.yaml

deploy: install
	kubectl apply -k config/manager
	kubectl apply -k config/rbac

undeploy:
	kubectl delete -k config/manager
	kubectl delete -k config/rbac

deploy-sample:
	kubectl apply -k sample

undeploy-sample:
	kubectl delete -k sample

uninstall: undeploy
	kubectl delete -f config/crd/bases/applications.deepshore.de_chirpstacks.yaml

bundle:
	operator-sdk generate kustomize manifests --package chirpstack-operator -q
	kustomize build config/manifests | operator-sdk generate bundle -q --overwrite --version 0.1.0
	operator-sdk bundle validate ./bundle

bundle-image: bundle
	docker buildx build --tag $(BUNDLE_IMAGE) -f bundle.Dockerfile . &&

clean:
	rm -fr bundle*
