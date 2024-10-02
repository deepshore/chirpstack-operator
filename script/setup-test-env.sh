#!/bin/bash

minikube start &&
  kubectl apply -f tests/manifests/redis.yaml &&
  kubectl apply -f tests/manifests/postgresql.yaml &&
  kubectl apply -f tests/manifests/mosquitto.yaml &&
  eval "$(minikube docker-env --shell=bash)" &&
  make docker-build
