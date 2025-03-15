#ifndef RESOURCE_LOADER_HELPER_HPP
#define RESOURCE_LOADER_HELPER_HPP

#include <godot_cpp/classes/resource.hpp>
#include <godot_cpp/classes/resource_loader.hpp>
#include <godot_cpp/templates/hash_map.hpp>
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/godot.hpp>

namespace godot {

class ResourceLoaderHelper {
public:
    template <typename T>
    static Ref<T> load_cached(const String &path, const String &resource_name = "Resource");

private:
    static HashMap<String, Ref<Resource>> cache;
};

} // namespace godot

#endif // RESOURCE_LOADER_HELPER_HPP
