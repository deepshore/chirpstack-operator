#!/bin/sh

kubectl port-forward --address 0.0.0.0 svc/prometheus-operated -n monitoring 9090
