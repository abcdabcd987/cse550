#include "disk_io.hpp"
#include <cstring>
#include <cstdint>
#include <sys/types.h>
#include <sys/stat.h>
#include <sys/mman.h>
#include <fcntl.h>
#include <unistd.h>

const char* ConcurrentQueue::STOP_SIGN = "\x01\xe4\x84\xaf\xe2\x21\x36\xa8";

void *disk_io_thread(void *raw_args) {
    const char* ERROR_OPEN = "file not found\n";
    const char* ERROR_STAT = "failed to fstat\n";
    const char* ERROR_MMAP = "failed to mmap\n";
    DiskIOArgs& args = *reinterpret_cast<DiskIOArgs*>(raw_args);
    for (;;) {
        auto filename = args.queue.get();
        if (filename == ConcurrentQueue::STOP_SIGN)
            break;

        const char* error = nullptr;
        const void* src = nullptr;
        ssize_t size = 0;
        do {
            int fd = open(filename.c_str(), O_RDONLY, 0);
            if (fd < 0) {
                error = ERROR_OPEN;
                break;
            }
            
            struct stat sbuf;
            if (fstat(fd, &sbuf) < 0) {
                error = ERROR_STAT;
                break;
            }
            size = sbuf.st_size;

            src = mmap(NULL, sbuf.st_size, PROT_READ, MAP_PRIVATE, fd, 0);
            if (src == MAP_FAILED) {
                error = ERROR_MMAP;
                break;
            }
        } while (0);

        if (error) {
            size = strlen(error);
            src = error;
        }

        auto buf = std::make_shared<std::vector<char>>(size);
        memmove(buf->data(), src, size);

        args.cache.put(filename, std::move(buf));
        uint64_t val = 1;
        write(args.event_fd, &val, sizeof(val));

        if (!error && munmap(const_cast<void*>(src), size) < 0) {
            perror("munmap");
        }
    }
}