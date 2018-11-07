#!/bin/bash

cd "$(dirname "$0")"
SESSION=paxos550_server

if [[ $# -ne 1 ]] ; then
    echo "USAGE: $0 NUMBER_OF_SERVERS"
    exit 0
fi

tmux new-session -d -s $SESSION -x 165 -y 60
for i in `seq 2 $1`; do
    tmux split-window -v -d -t $SESSION
done
tmux select-layout -t $SESSION tiled

for i in `seq 1 $1`; do
    ARGS="--id server$i --listen 0.0.0.0:$(( 9000 + $i ))"
    for j in `seq 1 $1`; do
        if [[ $i -ne $j ]]; then
            ARGS="$ARGS --peer server$j=127.0.0.1:$(( 9000 + $j ))"
        fi
    done
    tmux send-keys -t $SESSION.$(( i - 1 )) "../target/debug/server $ARGS" C-m
done

tmux attach -t $SESSION
