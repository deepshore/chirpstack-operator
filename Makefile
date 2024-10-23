COPY_MANIFESTS := role.yaml role_binding.yaml

default: manifests

manifest:
	mkdir -p manifest

$(addprefix manifest/,$(COPY_MANIFESTS)): manifest
	cp src/$@ $@

manifest/crd.yaml: manifests src/crd.rs
	cargo build
	cargo run --bin make-crd-manifest | yq -o yaml -P > manifest/crd.yaml || rm -f manifest/crd.yaml

manifests: $(addprefix manifest/,$(COPY_MANIFESTS)) manifest/crd.yaml

run-controller: install
	cargo build
	RUST_LOG=debug cargo run --bin controller

install: manifest/crd.yaml
	kubectl apply -f manifest/crd.yaml

deploy: install
	kubectl apply -k sample

undeploy:
	kubectl delete -k sample

uninstall: undeploy
	kubectl delete -f manifest/crd.yaml

clean:
	rm -fr manifest
