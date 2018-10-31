#pragma once

#include <unordered_map>
#include <list>
#include <vector>
#include <memory>
#include <iterator>
#include <mutex>

// ref: https://github.com/lamerman/cpp-lru-cache
class Cache {
public:
    using ItemType = std::shared_ptr<std::vector<char>>;

private:
    struct ListElementType {
        std::string key;
        ItemType item;
        ListElementType(std::string k, ItemType i): key(std::move(k)), item(std::move(i)) {}
    };

    std::mutex mutex;
    size_t max_size, cache_size;
    std::list<ListElementType> list;
    std::unordered_map<std::string, std::list<ListElementType>::iterator> map;

public:
    Cache(size_t max_size_): max_size(max_size_), cache_size(0) {
    }

    void put(const std::string &key, ItemType item) {
        std::lock_guard<std::mutex> lock(mutex);
        auto it = map.find(key);
        if (it != map.end()) {
            cache_size -= it->second->item->size();
            map.erase(it);
            list.erase(it->second);
        }

        cache_size += item->size();
        list.emplace_front(key, std::move(item));
        map[key] = list.begin();

        while (cache_size > max_size) {
            auto it = std::prev(list.end());
            map.erase(it->key);
            list.pop_back();
        }
    }

    ItemType get(const std::string &key) {
        std::lock_guard<std::mutex> lock(mutex);
        auto it = map.find(key);
        if (it == map.end()) {
            return ItemType();
        }
        list.splice(list.begin(), list, it->second);
        return it->second->item;
    }
};
