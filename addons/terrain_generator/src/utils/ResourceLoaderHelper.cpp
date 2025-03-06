#include "ResourceLoaderHelper.hpp"
#include "core/print_string.hpp"
#include <godot_cpp/classes/noise_texture2d.hpp>
#include <godot_cpp/classes/shader.hpp>
#include <godot_cpp/templates/hash_map.hpp>



namespace godot {

HashMap<String, Ref<Resource>> ResourceLoaderHelper::cache = HashMap<String, Ref<Resource>>();

template <typename T>
Ref<T> ResourceLoaderHelper::load_cached(const String &path, const String &resource_name) {
    if (cache.has(path)) {
        Ref<T> cached = cache[path];
        if (cached.is_valid()) {
            godot::print_line("✅ Using cached ", resource_name, ": ", path);
            return cached;
        } else {
            godot::print_line("⚠️ Cached ", resource_name, " at ", path, " is invalid. Reloading...");
        }
    }

    Ref<T> resource = ResourceLoader::get_singleton()->load(path);
    if (resource.is_valid()) {
        cache[path] = resource;
        godot::print_line("✅ Loaded and cached ", resource_name, ": ", path);
    } else {
        godot::print_line("❌ Failed to load ", resource_name, ": ", path);
    }

    return resource;
}

// Explicit template instantiation
template Ref<NoiseTexture2D> ResourceLoaderHelper::load_cached(const String&, const String&);
template Ref<Texture2D> ResourceLoaderHelper::load_cached(const String&, const String&);
template Ref<Shader> ResourceLoaderHelper::load_cached(const String&, const String&);

} // namespace godot
