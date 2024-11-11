OPERATOR_IMAGE := chirpstack-operator:latest
BUNDLE_IMAGE := chirpstack-operator-bundle:v0.1.0
REGISTRY := ghcr.io/deepshore

default: image bundle-image

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
	operator-sdk run bundle ${REGISTRY}/${BUNDLE_IMAGE} --namespace operators --timeout 5m0s

.PHONY: test
test:
	sh test/script/setup.sh
	which blackjack || cargo install mrblackjack
	BLACKJACK_LOG_LEVEL=blackjack=info blackjack --parallel 4 test/blackjack

test-from-ghcr:
	sh test/script/setup-from-ghcr.sh
	which blackjack || cargo install mrblackjack
	BLACKJACK_LOG_LEVEL=blackjack=info blackjack --parallel 4 test/blackjack

test-debug:
	sh test/script/setup.sh
	which blackjack || cargo install mrblackjack
	BLACKJACK_LOG_LEVEL=blackjack=debug blackjack --parallel 4 test/blackjack

clean:
	rm -fr bundle*
	minikube delete
