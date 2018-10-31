#include "http.hpp"
#include "network.hpp"
#include "parser.hpp"
#include <sstream>
#include <iostream>
#include <unordered_map>
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/types.h>
#include <sys/sendfile.h>
#include <sys/stat.h>
#include <unistd.h>

void WebServer::close_request(HTTPRequest *r) {
    if (close(r->fd_socket) < 0)
        perror("close");
}

enum DoRequestState {
    DOREQUEST_STATE_READ = 0,
    DOREQUEST_STATE_SERVE_STATIC,
    DOREQUEST_STATE_SENDFILE,
    DOREQUEST_STATE_FINISHING,
    DOREQUEST_STATE_LOOP,
    DOREQUEST_STATE_CLOSE,
    DOREQUEST_STATE_READ_AGAIN,
};

void WebServer::do_request_read(HTTPRequest *r) {
    size_t buf_remain = HTTPRequest::BUF_SIZE - (r->buf_tail - r->buf_head) - 1;
    buf_remain = std::min(buf_remain, HTTPRequest::BUF_SIZE - r->buf_tail % HTTPRequest::BUF_SIZE);
    char *ptail = &r->buf[r->buf_tail % HTTPRequest::BUF_SIZE];
    int nread = read(r->fd_socket, ptail, buf_remain);
    if (nread < 0) {
        // If errno == EAGAIN, that means we have read all
        // data. So go back to the main loop.
        if (errno != EAGAIN) {
            r->do_request_state = DOREQUEST_STATE_CLOSE;
            return;
        }
        r->do_request_state = DOREQUEST_STATE_READ_AGAIN;
        return;
    } else if (nread == 0) {
        // End of file. The remote has closed the connection.
        r->do_request_state = DOREQUEST_STATE_CLOSE;
        return;
    }

    r->buf_tail += nread;
    ParseResult parse_result = parse(r);
    if (parse_result == PARSE_RESULT_AGAIN) {
        return;
    }
    r->do_request_state = DOREQUEST_STATE_SERVE_STATIC;
}

void WebServer::serve_static(HTTPRequest *r) {
    std::string filename = r->uri;

    auto cache = io_cache.get(filename);
    if (cache) {
        if (enable_tcp_nodelay)
            tcp_nodelay_on(r->fd_socket);
        if (enable_tcp_cork)
            tcp_cork_on(r->fd_socket);

        r->do_request_state = DOREQUEST_STATE_SENDFILE;
        r->file_size = cache->size();
        r->cached_content = std::move(cache);
        r->writen = 0;
        r->offset = 0;
        r->buf_head = 0;
        r->buf_tail = 0;
    } else {
        io_task_queue.put(filename);
    }
}

void WebServer::serve_static_sendfile(HTTPRequest *r) {
    char *base = r->cached_content->data();
    size_t &writen = r->writen;
    while (writen < r->file_size) {
        char *start = base + writen;
        ssize_t n = write(r->fd_socket, (void*)start, r->file_size - writen);
        if (n < 0) {
            if (errno == EAGAIN)
                return;
            perror("write");
            r->do_request_state = DOREQUEST_STATE_READ;
            return;
        }
        writen += n;
    }

    if (enable_tcp_cork)
        tcp_cork_off(r->fd_socket); // send messages out
    r->do_request_state = DOREQUEST_STATE_FINISHING;
    return;
}

WebServer::DoRequestResult WebServer::do_request(HTTPRequest *r) {
    for (;;) {
        switch (r->do_request_state) {
            case DOREQUEST_STATE_READ:
                do_request_read(r);
                break;
            case DOREQUEST_STATE_SERVE_STATIC:
                serve_static(r);
                if (r->do_request_state == DOREQUEST_STATE_SERVE_STATIC)
                    return DO_REQUEST_WAIT_DISK;
                break;
            case DOREQUEST_STATE_SENDFILE:
                serve_static_sendfile(r);
                if (r->do_request_state == DOREQUEST_STATE_SENDFILE)
                    return DO_REQUEST_WRITE_AGAIN;
                break;
            case DOREQUEST_STATE_FINISHING:
                r->clear();
                r->do_request_state = DOREQUEST_STATE_CLOSE;
                break;
            case DOREQUEST_STATE_READ_AGAIN:
                r->do_request_state = DOREQUEST_STATE_READ;
                return DO_REQUEST_READ_AGAIN;
            case DOREQUEST_STATE_CLOSE:
                r->do_request_state = DOREQUEST_STATE_READ;
                return DO_REQUEST_CLOSE;
            case DOREQUEST_STATE_LOOP:
                break;
            default:
                break;
        }
    }
}
