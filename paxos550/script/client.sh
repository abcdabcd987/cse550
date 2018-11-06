#!/bin/bash
cd "$(dirname "$0")"

if [[ $# -ne 2 ]] ; then
    echo "USAGE: $0 CLIENT_NAME NUMBER_OF_SERVERS"
    exit 0
fi

ARGS=""
for i in `seq 1 $2`; do
    ARGS="$ARGS --server server$i=127.0.0.1:$(( 9000 + i ))"
done

../target/debug/client --id $1 $ARGS
