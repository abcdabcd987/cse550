#!/bin/bash

random_string() {
    # https://gist.github.com/earthgecko/3089509
    cat /dev/urandom | env LC_CTYPE=C tr -dc 'a-zA-Z0-9' | fold -w 8 | head -n 1
}

cd "$(dirname "$0")"

if [[ $# -ne 1 ]] ; then
    echo "USAGE: $0 NUMBER_OF_SERVERS"
    exit 0
fi

ARGS=""
for i in `seq 1 $1`; do
    ARGS="$ARGS --server server$i=127.0.0.1:$(( 9000 + i ))"
done

PREFIX=$(random_string)

for i in `seq 1 10`; do
    echo LOCK concurrent-$PREFIX-$i | ../target/debug/client --id client$i $ARGS > /dev/null 2>&1 &
done

wait
