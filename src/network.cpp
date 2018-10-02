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
#include <netinet/tcp.h>

int create_and_bind(int port, bool reuseport) {
    struct addrinfo hints;
    struct addrinfo *result, *rp;
    int s, sfd;
    char strport[10];
    snprintf(strport, 10, "%d", port);

    memset(&hints, 0, sizeof (struct addrinfo));
    hints.ai_family = AF_UNSPEC;     // Return IPv4 and IPv6 choices
    hints.ai_socktype = SOCK_STREAM; // We want a TCP socket
    hints.ai_flags = AI_PASSIVE;     // All interfaces

    s = getaddrinfo(NULL, strport, &hints, &result);
    if (s != 0) {
        fprintf(stderr, "getaddrinfo: %s\n", gai_strerror(s));
        abort();
    }

    for (rp = result; rp != NULL; rp = rp->ai_next) {
        sfd = socket(rp->ai_family, rp->ai_socktype, rp->ai_protocol);
        if (sfd == -1)
            continue;

        int optval = 1;
        if (setsockopt(sfd, SOL_SOCKET, reuseport ? SO_REUSEPORT : SO_REUSEADDR, &optval, sizeof(int)) < 0)
            continue;
  
        s = bind(sfd, rp->ai_addr, rp->ai_addrlen);
        if (s == 0) {
            /* We managed to bind successfully! */
            break;
        }
  
        close(sfd);
    }

    if (rp == NULL) {
        fprintf(stderr, "Could not bind\n");
        abort();
    }

    freeaddrinfo(result);

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
    // fprintf(stderr, "accept  fd=%d\n", infd);
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
