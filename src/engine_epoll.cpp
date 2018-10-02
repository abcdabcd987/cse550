#include <list>
#include <cstring>
#include <cstdlib>
#include <errno.h>
#include <fcntl.h>
#include <netdb.h>
#include <sys/socket.h>
#include <sys/epoll.h>
#include <sys/eventfd.h>
#include <unistd.h>
#include "http.hpp"
#include "network.hpp"
#include "http.hpp"
#include "disk_io.hpp"

void WebServer::run(int sfd, int backlog, int num_worker) {
    enum class EventSource {
        ListenFD = 1,
        AcceptFD,
        EventFD
    };

    struct EventContextBase {
        EventSource source;
    };
    struct EventContextAcceptedFD {
        EventSource source;
        HTTPRequest req;
    };

    make_socket_non_blocking(sfd);

    int event_fd = eventfd(0, EFD_NONBLOCK);
    if (event_fd < 0) {
        perror("eventfd");
        abort();
    }

    DiskIOArgs disk_io_args = { io_cache, io_task_queue, event_fd };
    for (size_t i = 0; i < num_worker; ++i) {
        pthread_t thread;
        if (pthread_create(&thread, NULL, disk_io_thread, static_cast<void*>(&disk_io_args)) < 0) {
            perror("pthread_create");
            abort();
        }
    }

    struct epoll_event *events = static_cast<struct epoll_event*>(std::calloc(sizeof(struct epoll_event), backlog));
    int efd = epoll_create1(0);
    if (efd < 0) {
        perror("epoll_create1");
        abort();
    }

    struct epoll_event event;
    {
        auto ctx = new EventContextBase;
        ctx->source = EventSource::ListenFD;
        event.data.ptr = static_cast<void*>(ctx);
        event.events = EPOLLIN | EPOLLET;
        if (epoll_ctl(efd, EPOLL_CTL_ADD, sfd, &event) < 0) {
            perror("epoll_ctl");
            abort();
        }
    }
    {
        auto ctx = new EventContextBase;
        ctx->source = EventSource::EventFD;
        event.data.ptr = static_cast<void*>(ctx);
        event.events = EPOLLHUP | EPOLLERR | EPOLLIN | EPOLLET;
        if (epoll_ctl(efd, EPOLL_CTL_ADD, event_fd, &event) < 0) {
            perror("epoll_ctl");
            abort();
        }
    }

    std::list<EventContextAcceptedFD*> list_wait_disk;

    // the event loop
    for (;;) {
        bool disk_io_finished = false;
        int n = epoll_wait(efd, events, backlog, -1);
        fprintf(stderr, "debug: got %d events\n", n);
        for (int i = 0; i < n; ++i) {
            auto ctx = static_cast<EventContextBase*>(events[i].data.ptr);

            if ((events[i].events & EPOLLERR) ||
                (events[i].events & EPOLLHUP) ||
                (!(events[i].events & EPOLLIN) && !(events[i].events & EPOLLOUT)))
            {
                fprintf(stderr, "debug: i=%d err handle\n", i);
                // An error has occured on this fd, or the socket is not
                // ready for reading
                if (ctx->source == EventSource::AcceptFD) {
                    auto cx = reinterpret_cast<EventContextAcceptedFD*>(ctx);
                    close_request(&cx->req);
                    delete cx;
                } else {
                    delete ctx;
                }
                continue;
            }

            fprintf(stderr, "debug: i=%d ctx->source=%d\n", i, ctx->source);
            if (ctx->source == EventSource::ListenFD) {
                // We have a notification on the listening socket, which
                // means one or more incoming connections.
                for (;;) {
                    int infd = accept_connection(sfd);
                    if (infd < 0) break;

                    auto cx = new EventContextAcceptedFD;
                    cx->source = EventSource::AcceptFD;
                    cx->req.fd_socket = infd;
                    cx->req.fd_epoll = efd;
                    cx->req.do_request_state = 0;

                    event.data.ptr = static_cast<void*>(cx);
                    event.events = EPOLLIN | EPOLLET | EPOLLONESHOT;
                    if (epoll_ctl(efd, EPOLL_CTL_ADD, infd, &event) < 0) {
                        perror("epoll_ctl");
                        abort();
                    }
                }
            } else if (ctx->source == EventSource::AcceptFD) {
                auto cx = reinterpret_cast<EventContextAcceptedFD*>(ctx);
                auto r = &cx->req;

                // We have data on the fd waiting to be read. Read and
                // display it. We must read whatever data is available
                // completely, as we are running in edge-triggered mode
                // and won't get a notification again for the same
                // data.
                DoRequestResult res = do_request(r);
                switch (res) {
                    case DO_REQUEST_READ_AGAIN:
                    case DO_REQUEST_WRITE_AGAIN:
                        event.data.ptr = static_cast<void*>(r);
                        event.events = EPOLLET | EPOLLONESHOT;
                        if (res == DO_REQUEST_READ_AGAIN) event.events |= EPOLLIN;
                        if (res == DO_REQUEST_WRITE_AGAIN) event.events |= EPOLLOUT;
                        if (epoll_ctl(r->fd_epoll, EPOLL_CTL_MOD, r->fd_socket, &event) < 0) {
                            perror("epoll_ctl");
                            abort();
                        }
                        break;
                    case DO_REQUEST_WAIT_DISK:
                        list_wait_disk.emplace_back(cx);
                        break;
                    case DO_REQUEST_CLOSE:
                        // closing a file descriptor will cause it to be removed from all epoll sets automatically
                        // see: http://stackoverflow.com/questions/8707601/is-it-necessary-to-deregister-a-socket-from-epoll-before-closing-it
                        close_request(r);
                        delete cx;
                        break;
                    default:
                        fprintf(stderr, "should never reach this point\n");
                        abort();
                }
            } else if (ctx->source == EventSource::EventFD) {
                disk_io_finished = true;
            } else {
                fprintf(stderr, "should never reach this point. ctx->source=%d\n", ctx->source);
                abort();
            }
        }

        if (disk_io_finished) {
            for (auto it = list_wait_disk.begin(); it != list_wait_disk.end(); ) {
                bool remove = true;
                auto cx = *it;
                auto r = &cx->req;
                DoRequestResult res = do_request(r);
                switch (res) {
                    case DO_REQUEST_READ_AGAIN:
                    case DO_REQUEST_WRITE_AGAIN:
                        event.data.ptr = static_cast<void*>(r);
                        event.events = EPOLLET | EPOLLONESHOT;
                        if (res == DO_REQUEST_READ_AGAIN) event.events |= EPOLLIN;
                        if (res == DO_REQUEST_WRITE_AGAIN) event.events |= EPOLLOUT;
                        if (epoll_ctl(r->fd_epoll, EPOLL_CTL_MOD, r->fd_socket, &event) < 0) {
                            perror("epoll_ctl");
                            abort();
                        }
                        break;
                    case DO_REQUEST_WAIT_DISK:
                        remove = false;
                        break;
                    case DO_REQUEST_CLOSE:
                        // closing a file descriptor will cause it to be removed from all epoll sets automatically
                        // see: http://stackoverflow.com/questions/8707601/is-it-necessary-to-deregister-a-socket-from-epoll-before-closing-it
                        close_request(r);
                        delete cx;
                        break;
                    default:
                        fprintf(stderr, "should never reach this point\n");
                        abort();
                }
                if (remove) {
                    list_wait_disk.erase(it++);
                } else {
                    ++it;
                }
            }
        }
    }
}
