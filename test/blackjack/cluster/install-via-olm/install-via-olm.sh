#!/bin/sh

. ./lib.sh

olm_install_local_registry || { sleep 10; olm_install_local_registry; }
