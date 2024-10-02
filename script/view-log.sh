#!/bin/sh -l

tmp_file="$(mktemp)" &&
  pod_name="$(kubectl get pods -n chirpstack-operator-system -o yaml | yq '.items[0].metadata.name')" &&
  test -n "$pod_name" &&
  kubectl logs pod/"$pod_name" -n chirpstack-operator-system > "$tmp_file" &&
  { < /dev/tty vi "$tmp_file" > /dev/tty | true; } &&
  rm "$tmp_file"

    #| while read f ; do echo -n "$f" | yq -P -o json >> "$tmp_file" ; done
