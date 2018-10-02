#include <string>
#include <vector>
#include <iostream>

#include "parser.hpp"

ParseResult parse(HTTPRequest *r) {
    enum ParseState {
        P_URI = 0,
        P_DONE
    };

    if (r->parser_state != P_DONE) {
        for (size_t i = r->buf_head; i < r->buf_tail; ++i) {
            char ch = r->buf[i % HTTPRequest::BUF_SIZE];
            if (ch == '\n') {
                r->parser_state = P_DONE;
                break;
            }
            r->uri += ch;
        }
        r->buf_head = r->buf_tail;
    }

    return r->parser_state == P_DONE ? PARSE_RESULT_OK : PARSE_RESULT_AGAIN;
}
