CXXFLAGS += -Isrc/ -Itest/ -g -std=c++11 -pthread #-O3
LDFLAGS  += -g

SRCS = \
	src/disk_io.cpp \
	src/engine_epoll.cpp \
	src/http.cpp \
	src/http_request.cpp \
	src/network.cpp \
	src/parser.cpp

MAIN_SRCS = \
	src/main.cpp \

OBJS = $(subst .cpp,.o,$(SRCS))
MAIN_OBJS = $(subst .cpp,.o,$(MAIN_SRCS))

.PHONY: all makedir main clean dist-clean

all: makedir main

makedir:
	mkdir -p bin

main: makedir bin/naughttpd

bin/naughttpd: $(OBJS) src/main.o
	$(CXX) $(CXXFLAGS) $(LDFLAGS) -o $@ $^ $(LDLIBS)

depend: .depend
.depend: $(shell find . -name '*.cpp')
	rm -f ./.depend
	$(CXX) $(CXXFLAGS) -MM $^ >> ./.depend

clean:
	rm -f $(OBJS)

dist-clean: clean
	rm -f *~ .depend

include .depend
