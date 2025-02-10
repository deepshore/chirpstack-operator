export REGISTRY := ghcr.io/deepshore
export VERSION ?= $(shell git describe --tags)
export OPERATOR_IMAGE := chirpstack-operator:$(VERSION)
export BUNDLE_IMAGE := chirpstack-operator-bundle:$(VERSION)
export MINIKUBE_CPUS ?= 4
export MINIKUBE_MEM ?= 2GB

.PHONY: build run image bundle-image push-images deploy install-blackjack \
	test-prepare test-prepare-full test-prepare-with-local-controller \
	test-prepare-with-olm-local-registry test-prepare-with-olm-ghcr-registry \
	test-with-local-controller test-with-olm-local-registry test-with-olm-ghcr-registry \
	clean clean-all

export BLACKJACK_LOG_LEVEL ?= blackjack=info
export BLACKJACK_SETTINGS ?= --user-parallel $(MINIKUBE_CPUS) --cluster-parallel 1 \
	--timeout-scaling 2 --user-attempts 2 --cluster-attempts 1

default: build config/crd/bases/applications.deepshore.de_chirpstacks.yaml

Cargo.lock: Cargo.toml chirpstack-operator/Cargo.toml droperator/Cargo.toml
	cargo update

build:
	cargo test --color always 2>&1 | less -R

run:
	kubectl apply -k config/crd
	RUST_LOG=info cargo run --bin controller

config/crd/bases/applications.deepshore.de_chirpstacks.yaml: chirpstack-operator/src/crd.rs
	cargo run --bin make-crd-manifest | yq -o yaml -P > $@ || rm -f $@

image: Cargo.lock
	docker buildx build --tag $(REGISTRY)/$(OPERATOR_IMAGE) --platform linux/amd64,linux/arm64 -f Dockerfile .

bundle: config/crd/bases/applications.deepshore.de_chirpstacks.yaml Cargo.toml
	cd config/manager && \
		kustomize edit set image chirpstack-operator=$(REGISTRY)/$(OPERATOR_IMAGE)
	operator-sdk generate kustomize manifests --package chirpstack-operator -q
	kustomize build config/manifests | \
		operator-sdk generate bundle -q --overwrite --version $(subst v,,$(VERSION))
	operator-sdk bundle validate ./bundle
	echo \
		"LABEL org.opencontainers.image.source=https://github.com/deepshore/chirpstack-operator.git" \
		>> bundle.Dockerfile

bundle-image: bundle
	docker buildx build --tag $(REGISTRY)/$(BUNDLE_IMAGE) --platform linux/amd64,linux/arm64 -f bundle.Dockerfile .

push-images: image bundle-image
	docker push $(REGISTRY)/$(OPERATOR_IMAGE)
	docker push $(REGISTRY)/$(BUNDLE_IMAGE)

deploy:
	operator-sdk run bundle $(REGISTRY)/$(BUNDLE_IMAGE) --namespace operators --timeout 5m0s

install-blackjack:
	which blackjack || cargo install mrblackjack

test-prepare: Cargo.lock install-blackjack \
	config/crd/bases/applications.deepshore.de_chirpstacks.yaml

test-prepare-full: clean test-prepare
	. ./test/script/lib.sh && olm_prepare_install_local_registry

test-prepare-with-local-controller: test-prepare
	cargo build --bin controller
	. ./test/script/lib.sh && olm_remove_operator && start_minikube
	kubectl apply -k config/crd

test-prepare-with-olm-local-registry: test-prepare
	. ./test/script/lib.sh && olm_prepare_install_local_registry

test-prepare-with-olm-ghcr-registry: test-prepare
	. ./test/script/lib.sh && { olm_install_operator $(REGISTRY)/$(BUNDLE_IMAGE) || { sleep 10; olm_install_operator ${REGISTRY}/${BUNDLE_IMAGE}; } }

test-with-local-controller: test-prepare-with-local-controller
	sh test/script/run-test-with-local-controller.sh; kubectl delete -k config/crd

test-with-olm-local-registry: test-prepare-with-olm-local-registry
	blackjack $(BLACKJACK_SETTINGS) test/blackjack/cluster
	blackjack $(BLACKJACK_SETTINGS) test/blackjack/user

test-with-olm-ghcr-registry: test-prepare-with-olm-ghcr-registry
	blackjack $(BLACKJACK_SETTINGS) test/blackjack/cluster/add-prometheus
	blackjack $(BLACKJACK_SETTINGS) test/blackjack/user

clean:
	rm -fr bundle*

clean-all: clean
	minikube delete || true
	docker system prune -af || true
