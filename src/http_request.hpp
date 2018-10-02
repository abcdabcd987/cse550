#pragma once
#include <map>
#include <vector>
#include <string>
#include <memory>
#include <ostream>
#include "util.hpp"

enum HTTPMethod {
    HTTP_METHOD_UNKNOWN,
    HTTP_METHOD_GET,
    HTTP_METHOD_POST
};

struct HTTPRequest {
    // constants
    static constexpr size_t BUF_SIZE = 1024;

    // buffer
    char buf[BUF_SIZE];
    size_t buf_head;
    size_t buf_tail;

    // parser temporary variables
    int parser_state;

    // request info
    std::string uri;

    // engine data
    int fd_socket;
    int fd_epoll;

    // do_request() internal variables
    int do_request_state;
    std::shared_ptr<std::vector<char>> cached_content;
    size_t file_size;
    size_t writen;
    off_t offset;
    size_t readn;

    // funcs
    HTTPRequest();
    void clear();
};

std::ostream &operator<<(std::ostream &out, const HTTPRequest &r);
