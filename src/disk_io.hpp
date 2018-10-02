#pragma once
#include "cache.hpp"
#include "concurrent_queue.hpp"

struct DiskIOArgs {
    Cache &cache;
    ConcurrentQueue &queue;
    int event_fd;
};

void *disk_io_thread(void *raw_args);
