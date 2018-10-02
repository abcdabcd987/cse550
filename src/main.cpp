#include <cstdio>
#include <cstring>
#include <getopt.h>
#include <signal.h>
#include <sys/types.h>
#include <sys/socket.h>
#include "http.hpp"
#include "network.hpp"

int main(int argc, char *argv[]) {
    if (argc != 3) {
        printf("usage: %s addr port\n", argv[0]);
        return 1;
    }
    const char* addr = argv[1];
    const int port = std::stoi(argv[2]);
    const int backlog = 511;
    const int num_worker = 4;

    // listen
    signal(SIGPIPE, SIG_IGN);
    int sfd = -1;
    sfd = create_and_bind(port, false);
    if (listen(sfd, backlog) < 0) {
        perror("listen");
        abort();
    }

    WebServer server;
    server.enable_tcp_nodelay = true;
    server.enable_tcp_cork = false;
    server.run(sfd, backlog, num_worker);
}
