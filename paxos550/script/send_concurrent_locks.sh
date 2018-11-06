#!/bin/bash
cd "$(dirname "$0")"

if [[ $# -ne 1 ]] ; then
    echo "USAGE: $0 NUMBER_OF_SERVERS"
    exit 0
fi

ARGS=""
for i in `seq 1 $1`; do
    ARGS="$ARGS --server server$i=127.0.0.1:$(( 9000 + i ))"
done

for i in `seq 1 10`; do
    echo LOCK concurrent_lock_$i | ../target/debug/client --id client$i $ARGS > /dev/null 2>&1 &
done

wait
