DOCKER_IMAGE := chirpstack-controller:latest

default: build manifest

image:
	docker buildx build --tag $(DOCKER_IMAGE) .

manifest/crd.yaml: src/crd.rs
	cargo build
	cargo run --bin make-crd-manifest | yq -o yaml -P > manifest/crd.yaml || rm -f manifest/crd.yaml

build:
	cargo build

run-controller-local: install
	cargo build
	RUST_LOG=debug cargo run --bin controller

install: manifest/crd.yaml
	kubectl apply -f manifest/crd.yaml

deploy:
	kubectl apply -k manifest/manager

undeploy:
	kubectl delete -k manifest/manager

deploy-sample: install
	kubectl apply -k sample

undeploy-sample:
	kubectl delete -k sample

uninstall: undeploy-sample
	kubectl delete -f manifest/crd.yaml

clean:
	rm -fr manifest
