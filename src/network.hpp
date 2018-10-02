#pragma once
#include <sys/types.h>

int create_and_bind(int port, bool reuseport);
void make_socket_non_blocking(int sfd);
int accept_connection(int sfd);
void tcp_cork_on(int fd);
void tcp_cork_off(int fd);
void tcp_nodelay_on(int fd);
void tcp_nodelay_off(int fd);
