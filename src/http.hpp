#pragma once

#include "http_request.hpp"
#include "disk_io.hpp"
#include "concurrent_queue.hpp"

class WebServer {
    Cache io_cache{1<<30};
    ConcurrentQueue io_task_queue;

    enum DoRequestResult {
        DO_REQUEST_READ_AGAIN = 0,
        DO_REQUEST_WRITE_AGAIN,
        DO_REQUEST_WAIT_DISK,
        DO_REQUEST_CLOSE
    };

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

    void close_request(HTTPRequest *r);
    DoRequestResult do_request(HTTPRequest *r);
    bool do_request_accepted_fd(EventContextAcceptedFD *cx);
    void do_request_read(HTTPRequest *r);
    void serve_static(HTTPRequest *r);
    void serve_static_sendfile(HTTPRequest *r);

public:
    bool enable_tcp_cork;
    bool enable_tcp_nodelay;
    void run(int sfd, int backlog, int num_worker);
};

