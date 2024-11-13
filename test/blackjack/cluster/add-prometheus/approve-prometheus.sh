#!/bin/sh

name="$(kubectl get installplan -n operators -o yaml --selector operators.coreos.com/prometheus.operators= | yq '.items[0].metadata.name')" &&
kubectl -n operators patch installplan ${name} -p '{"spec":{"approved":true}}' --type merge
