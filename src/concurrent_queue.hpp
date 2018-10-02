#pragma once
#include <queue>
#include <thread>
#include <mutex>
#include <condition_variable>

// ref: https://juanchopanzacpp.wordpress.com/2013/02/26/concurrent-queue-c11/
class ConcurrentQueue {
    std::queue<std::string> queue;
    std::mutex mutex;
    std::condition_variable cv;

public:
    static const char* STOP_SIGN;
    std::string get() {
        std::unique_lock<std::mutex> lock(mutex);
        while (queue.empty())
            cv.wait(lock);
        auto item = queue.front();
        queue.pop();
        return item;
    }

    void put(std::string item) {
        mutex.lock();
        queue.emplace(std::move(item));
        mutex.unlock();
        cv.notify_one();
    }
};
