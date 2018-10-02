# 550server

## Compile

    make

## Run

    ./550server IP PORT

## Design

* All networking I/O is non-blocking.
* The main thread waits for events notified by `epoll`.
  It handles networking I/O asynchronously.
* A thread pool is spawned in the beginning to serve disk I/O.
* Each worker in the thread pool gets disk I/O tasks from the shared task queue.
  The worker then `mmap` the file and copy the content to a buffer.
  It blocks when the memory page is missing.
  After the copying finishes, the worker will put the content into the shared cache.
* Disk I/O workers notify the main thread by `eventfd`.
* The shared task queue is thread-safe, implemented using mutex and condition variable.
* The shared cache has a maximum size limit.
  We use the LRU algorithm to evict old entries.
  A mutex is used to support the concurrent access to the cache.
* There is a reference counting associated with each cached files.
  Cached files will free the memory only after no thread is using it.

## Reference

Most of the code are adopted from the hobby project Lequn wrote before.
See [naughttpd](https://github.com/abcdabcd987/naughttpd)
