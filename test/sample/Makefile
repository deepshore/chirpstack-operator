chirpstack-config.yaml: $(shell find chirpstack-config.d -type f)
	kubectl create configmap chirpstack-config --from-file=chirpstack-config.d --dry-run=client -o yaml > chirpstack-config.yaml
