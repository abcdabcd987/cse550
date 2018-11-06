#!/bin/bash
cd "$(dirname "$0")"
ARGS="--server server1=127.0.0.1:9001 \
      --server server2=127.0.0.1:9002 \
      --server server3=127.0.0.1:9003"

for i in `seq 1 10`; do
    echo LOCK concurrent_lock_$i | ../target/debug/client --id client$i $ARGS > /dev/null 2>&1 &
done

wait
