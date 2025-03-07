#include "ResourceLoaderHelper.hpp"
#include "core/print_string.hpp"
#include "godot_cpp/classes/fast_noise_lite.hpp"
#include <godot_cpp/classes/noise_texture2d.hpp>
#include <godot_cpp/classes/shader.hpp>
#include <godot_cpp/templates/hash_map.hpp>
#include <godot_cpp/classes/file_access.hpp>
#include <godot_cpp/classes/dir_access.hpp>

namespace godot {

HashMap<String, Ref<Resource>> ResourceLoaderHelper::cache = HashMap<String, Ref<Resource>>();

template <typename T>
Ref<T> ResourceLoaderHelper::load_cached(const String &path, const String &resource_name) {
    if (!FileAccess::file_exists(path)) {
        godot::print_line("❌ File does NOT exist: ", path);
        return Ref<T>();
    }

    if (cache.has(path)) {
        godot::print_line("🔍 Found cached resource for: ", path);
        Ref<Resource> cached = cache[path];
        if (cached.is_valid()) {
            Ref<T> typed_cached = cached;
            if (typed_cached.is_valid()) {
                godot::print_line("✅ Using valid cached ", resource_name, ": ", path);
                return typed_cached;
            } else {
                godot::print_line("⚠️ Cached resource at ", path, " has incorrect type. Removing from cache.");
                cache.erase(path);
            }
        } else {
            godot::print_line("⚠️ Cached resource at ", path, " is invalid. Removing from cache.");
            cache.erase(path);
        }
    }

    godot::print_line("📂 Loading resource from disk: ", path);
    Ref<Resource> resource = ResourceLoader::get_singleton()->load(path);
    if (resource.is_valid()) {
        Ref<T> typed_resource = resource;
        if (typed_resource.is_valid()) {
            cache[path] = resource;
            godot::print_line("✅ Successfully loaded and cached ", resource_name, " (", resource->get_class(), "): ", path);
            return typed_resource;
        } else {
            godot::print_line("❌ Loaded resource is wrong type. Expected ", resource_name, ", got ", resource->get_class());
        }
    } else {
        godot::print_line("❌ Failed to load resource from disk: ", path);
    }

    return Ref<T>();
}

// Explicit template instantiation
template Ref<NoiseTexture2D> ResourceLoaderHelper::load_cached(const String&, const String&);
template Ref<Texture2D> ResourceLoaderHelper::load_cached(const String&, const String&);
template Ref<Shader> ResourceLoaderHelper::load_cached(const String&, const String&);
template Ref<FastNoiseLite> ResourceLoaderHelper::load_cached(const String&, const String&);

} // namespace godot
