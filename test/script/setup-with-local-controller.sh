#!/bin/sh

. $(dirname $0)/lib.sh

olm_remove_operator &&
start_minikube
