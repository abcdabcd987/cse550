#!/bin/bash

cd "$(dirname "$0")"
SESSION=paxos550_server

tmux new-session -d -s $SESSION -x 165 -y 60
tmux split-window -v -d -t $SESSION
tmux split-window -v -d -t $SESSION
tmux select-layout -t $SESSION even-vertical

tmux send-keys -t $SESSION.0 "../target/debug/server --id server1 --listen 0.0.0.0:9001 --peer server2=127.0.0.1:9002 --peer server3=127.0.0.1:9003" C-m
tmux send-keys -t $SESSION.1 "../target/debug/server --id server2 --listen 0.0.0.0:9002 --peer server1=127.0.0.1:9001 --peer server3=127.0.0.1:9003" C-m
tmux send-keys -t $SESSION.2 "../target/debug/server --id server3 --listen 0.0.0.0:9003 --peer server1=127.0.0.1:9001 --peer server2=127.0.0.1:9002" C-m

tmux $1 attach -t $SESSION
