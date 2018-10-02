#include "http_request.hpp"

HTTPRequest::HTTPRequest() {
    clear();
}

void HTTPRequest::clear() {
    buf_head = 0;
    buf_tail = 0;
    parser_state = 0;
}
