#include "network.hpp"
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include <errno.h>
#include <fcntl.h>
#include <netdb.h>
#include <unistd.h>
#include <sys/socket.h>
#include <sys/epoll.h>
#include <arpa/inet.h>
#include <netinet/tcp.h>

int create_and_bind(const char* addr, int port) {
    int sfd = socket(AF_INET, SOCK_STREAM, 0);
    if (sfd < 0) {
        perror("socket");
        abort();
    }

    int optval = 1;
    if (setsockopt(sfd, SOL_SOCKET, SO_REUSEADDR, &optval, sizeof(int)) < 0) {
        perror("setsockopt");
        abort();
    }

    struct sockaddr_in sockaddr;
    memset(&sockaddr, 0, sizeof(sockaddr));
    sockaddr.sin_family = AF_INET;
    sockaddr.sin_port = htons(port);
    inet_pton(AF_INET, addr, &(sockaddr.sin_addr));

    int s = bind(sfd, (struct sockaddr*) &sockaddr, sizeof(sockaddr));
    if (s < 0) {
        perror("bind");
        fprintf(stderr, "could not bind.\n");
        abort();
    }

    return sfd;
}

void make_socket_non_blocking(int sfd) {
    int flags, s;

    flags = fcntl(sfd, F_GETFL, 0);
    if (flags == -1) {
        perror("fcntl");
        abort();
    }

    flags |= O_NONBLOCK;
    s = fcntl(sfd, F_SETFL, flags);
    if (s == -1) {
        perror("fcntl");
        abort();
    }
}

int accept_connection(int sfd) {
    int infd = accept4(sfd, NULL, NULL, SOCK_NONBLOCK);
    if (infd < 0) {
        if (errno == EAGAIN || errno == EWOULDBLOCK) {
            return -1;
        } else {
            perror("accept4");
            abort();
        }
    }
    return infd;
}

void set_tcp_opt(int fd, int opt, int optval) {
    if (setsockopt(fd, SOL_TCP, opt, &optval, sizeof(optval)) < 0) {
        perror("setsockopt");
        abort();
    }
}

void tcp_cork_on(int fd) {
    set_tcp_opt(fd, TCP_CORK, 1);
}

void tcp_cork_off(int fd) {
    set_tcp_opt(fd, TCP_CORK, 0);
}

void tcp_nodelay_on(int fd) {
    set_tcp_opt(fd, TCP_NODELAY, 1);
}

void tcp_nodelay_off(int fd) {
    set_tcp_opt(fd, TCP_NODELAY, 0);
}
