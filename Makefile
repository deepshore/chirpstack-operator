OPERATOR_IMAGE := chirpstack-operator:latest
BUNDLE_IMAGE := chirpstack-operator-bundle:v0.1.0
REGISTRY := ghcr.io/deepshore

default: build

build:
	cargo test

image:
	docker buildx build --tag $(REGISTRY)/$(OPERATOR_IMAGE) -f Dockerfile .

config/crd/bases/applications.deepshore.de_chirpstacks.yaml: chirpstack-operator/src/crd.rs
	cargo build
	cargo run --bin make-crd-manifest | yq -o yaml -P > $@ || rm -f $@

bundle: config/crd/bases/applications.deepshore.de_chirpstacks.yaml
	cd config/manager && kustomize edit set image chirpstack-operator=${REGISTRY}/chirpstack-operator
	operator-sdk generate kustomize manifests --package chirpstack-operator -q
	kustomize build config/manifests | operator-sdk generate bundle -q --overwrite --version 0.1.0
	operator-sdk bundle validate ./bundle
	echo "LABEL org.opencontainers.image.source=https://github.com/deepshore/chirpstack-operator.git" >> bundle.Dockerfile

bundle-image: bundle
	docker buildx build --tag $(REGISTRY)/$(BUNDLE_IMAGE) -f bundle.Dockerfile .

push-images: image bundle-image
	docker push $(REGISTRY)/$(OPERATOR_IMAGE)
	docker push $(REGISTRY)/$(BUNDLE_IMAGE)

deploy:
	operator-sdk run bundle $(REGISTRY)/$(BUNDLE_IMAGE) --namespace operators --timeout 5m0s

run:
	RUST_LOG=debug cargo run --bin controller

test-full: clean build config/crd/bases/applications.deepshore.de_chirpstacks.yaml
	sh test/script/prepare-with-olm-local-registry.sh
	which blackjack || cargo install mrblackjack
	BLACKJACK_LOG_LEVEL=blackjack=debug blackjack --parallel $(MINIKUBE_CPUS) --timeout-scaling 2 test/blackjack

test-with-local-controller: config/crd/bases/applications.deepshore.de_chirpstacks.yaml
	cargo build --bin controller
	which blackjack || cargo install mrblackjack
	sh test/script/setup-with-local-controller.sh
	kubectl apply -k config/crd
	sh test/script/run-test-with-local-controller.sh

.PHONY: test
test-with-olm-local-registry:
	sh test/script/setup-with-olm-local-registry.sh
	which blackjack || cargo install mrblackjack
	BLACKJACK_LOG_LEVEL=blackjack=info blackjack --parallel $(MINIKUBE_CPUS) --timeout-scaling 2 test/blackjack/user

test-with-olm-ghcr:
	sh test/script/setup-with-olm-ghcr.sh
	which blackjack || cargo install mrblackjack
	BLACKJACK_LOG_LEVEL=blackjack=info blackjack --parallel $(MINIKUBE_CPUS) --timeout-scaling 2 test/blackjack/user

clean:
	rm -fr bundle*

clean-minikube:
	minikube delete || true
