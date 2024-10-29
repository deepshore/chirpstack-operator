DOCKER_IMAGE := chirpstack-controller:latest

default: build manifest

image:
	docker buildx build --tag $(DOCKER_IMAGE) .

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

clean:
	make undeploy-sample; make undeploy; make uninstall
