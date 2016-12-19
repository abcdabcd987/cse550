#include <cstring>
#include <signal.h>
#include <sys/types.h>
#include <sys/socket.h>
#include "http.hpp"
#include "engines.hpp"
#include "network.hpp"

typedef void (*EngineFunction)(int, int);
struct EnginePair {
    const char *name;
    EngineFunction func;
};
const EnginePair engines[] = {
    { "naive" , engine_naive  },
    { "fork"  , engine_fork   },
    { "thread", engine_thread },
    { "pool"  , engine_pool   },
    { "select", engine_select },
    { "poll"  , engine_poll   },
    { "epoll" , engine_epoll  }
};

void help(int argc, char *argv[]) {
    fprintf(stderr, "usage: %s port engine\n", argv[0]);
    fprintf(stderr, "available engines:\n");
    for (size_t i = 0; i < sizeof(engines) / sizeof(EnginePair); ++i)
        fprintf(stderr, "    %s\n", engines[i].name);
    exit(EXIT_FAILURE);
}

int main(int argc, char *argv[]) {
    if (argc != 3)
        help(argc, argv);
    EngineFunction engine = NULL;
    for (size_t i = 0; i < sizeof(engines) / sizeof(EnginePair); ++i)
        if (strcmp(engines[i].name, argv[2]) == 0)
            engine = engines[i].func;
    if (!engine)
        help(argc, argv);

    // set up
    int backlog = 511;
    fprintf(stderr, "listen backlog = %d\n", backlog);
    signal(SIGPIPE, SIG_IGN);
    int sfd = create_and_bind(argv[1]);
    if (listen(sfd, backlog) < 0) {
        perror("listen");
        abort();
    }

    // run
    engine(sfd, backlog);
}
