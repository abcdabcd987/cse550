CSE550 Lock Service with Paxos
===============================

Author
-------
Lequn Chen, 1822436, lqchen@cs.washington.edu


Design & Implementation
------------------------
* Paxos
  * We decided to implement Paxos as minimal as possible to demonstrate that
    Paxos can maintain its safety guarantees as long as the core ideas of Paxos
    are implemented correctly.
  * No leader (i.e. distinguished proposer/learner).
  * No Nack message.
  * Learners need to learn the value from Acceptors once the learner receive
    the Accepted messages from the majority.
  * Use an method similar to the exponential back-off when timeout happens.
  * Run Proposer, Acceptor, and Learner in the same process.
  * Modular design that returns message structs after each operation instead of
    doing real networking I/O, so that it is easy to write unit test for
    components (i.e. Proposer, Acceptor, and Learner).
* Server
  * Single-threaded
  * Event-driven
  * Non-blocking networking I/O
  * Communicate with peer servers and clients via UDP
* Client
  * Shell-like
  * Randomly choose a server to send messages to.
  * There is no response after a `LOCK` or `UNLOCK` operation has sent to the
    server. Clients need to use `LOG` or `LOCKS` to check whether the operation
    is successful or not.
* Known limitations
  * Servers that are isolated during network partition cannot make new progress
    after the network recovers from the partition.
  * No recovery for restarted servers.


Compilation
------------
    # Install latest nightly Rust
    curl https://sh.rustup.rs -sSf | sh -- --default-toolchain nightly
    # Compile
    cargo build


Run
----
The compiled binary locates at
* ./target/debug/server
* ./target/debug/client

You can also refer to the scripts located at
* ./script/tmux_start_servers.sh
* ./script/send_concurrent_locks.sh
* ./script/client.sh


Example
--------
1. Run `./script/tmux_start_servers.sh 5` to start 5 servers.
2. Run `./script/send_concurrent_locks.sh 5` to send concurrent lock requests.
3. Run `./script/client.sh 5` and use commands like `LOG server1`
   to check that each server has exactly the same log.
4. Kill the last two servers.
5. Run `./script/send_concurrent_locks.sh 3` to send concurrent lock requests.
6. Run `./script/client.sh 3` and use commands like `LOG server1`
   to check that each server made progress and has exactly the same log.
7. Kill the last one servers.
8. Run `./script/send_concurrent_locks.sh 2` to send concurrent lock requests.
9. Run `./script/client.sh 2` and use commands like `LOG server1`
   to check that there is no new progress made.
