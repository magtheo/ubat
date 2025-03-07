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
    // File existence check
    if (FileAccess::file_exists(path)) {
        godot::print_line("✅ File exists: ", path);
    } else {
        godot::print_line("❌ File does NOT exist: ", path);
        return Ref<T>();
    }
    
    // If in cache, return cached version with safety checks
    if (cache.has(path)) {
        godot::print_line("🔍 Cache entry found for: ", path);
        Ref<Resource> cached_resource = cache[path];
        
        if (cached_resource.is_valid()) {
            godot::print_line("🔍 Cache entry is valid");
            // Perform the safest possible cast
            Ref<T> typed_resource;
            typed_resource.instantiate();
            if (Object::cast_to<T>(*cached_resource)) {
                typed_resource = cached_resource;
                godot::print_line("✅ Using cached ", resource_name, ": ", path);
                return typed_resource;
            } else {
                godot::print_line("⚠️ Cache type mismatch for ", resource_name);
                cache.erase(path); // Remove invalid entry
            }
        } else {
            godot::print_line("⚠️ Cached resource is invalid for ", path);
            cache.erase(path); // Remove invalid entry
        }
    }

    // Not in cache or invalid cache, try to load
    godot::print_line("📂 Attempting to load: ", path);
    
    // Extra safety: Try to use a different loading approach
    godot::print_line("🔍 Try loading with ResourceLoader::load_threaded_request");
    Error err = ResourceLoader::get_singleton()->load_threaded_request(path, "NoiseTexture2D");
    if (err != OK) {
        godot::print_line("❌ Error initiating load: ", err);
        
        // Try alternative loading method
        godot::print_line("🔍 Try alternative loading with ResourceLoader::load");
        Ref<Resource> resource = ResourceLoader::get_singleton()->load(path);
        if (resource.is_valid()) {
            godot::print_line("✅ Alternative load succeeded");
            
            // Safety type check
            Ref<T> typed_resource;
            typed_resource.instantiate();
            if (Object::cast_to<T>(*resource)) {
                typed_resource = resource;
                cache[path] = resource;
                godot::print_line("✅ Loaded and cached ", resource_name, ": ", path);
                return typed_resource;
            } else {
                godot::print_line("❌ Loaded resource is wrong type: ", resource->get_class());
            }
        } else {
            godot::print_line("❌ Alternative load failed");
        }
        return Ref<T>();
    }
    
    // Wait for loading to complete
    godot::print_line("🔍 Waiting for threaded load to complete");
    ResourceLoader::ThreadLoadStatus status = ResourceLoader::THREAD_LOAD_IN_PROGRESS;
    while (status == ResourceLoader::THREAD_LOAD_IN_PROGRESS) {
        status = ResourceLoader::get_singleton()->load_threaded_get_status(path);
    }
    
    godot::print_line("🔍 Thread load status: ", status);
    if (status == ResourceLoader::THREAD_LOAD_LOADED) {
        Ref<Resource> resource = ResourceLoader::get_singleton()->load_threaded_get(path);
        if (resource.is_valid()) {
            godot::print_line("✅ Threaded load succeeded");
            
            // Safety type check
            Ref<T> typed_resource;
            typed_resource.instantiate();
            if (Object::cast_to<T>(*resource)) {
                typed_resource = resource;
                cache[path] = resource;
                godot::print_line("✅ Loaded and cached ", resource_name, ": ", path);
                return typed_resource;
            } else {
                godot::print_line("❌ Loaded resource is wrong type: ", resource->get_class());
            }
        } else {
            godot::print_line("❌ Threaded load succeeded but resource is null");
        }
    } else {
        godot::print_line("❌ Threaded load failed with status: ", status);
    }
    
    return Ref<T>();
}

// Explicit template instantiation
template Ref<NoiseTexture2D> ResourceLoaderHelper::load_cached(const String&, const String&);
template Ref<Texture2D> ResourceLoaderHelper::load_cached(const String&, const String&);
template Ref<Shader> ResourceLoaderHelper::load_cached(const String&, const String&);
template Ref<FastNoiseLite> ResourceLoaderHelper::load_cached(const String&, const String&);

} // namespace godot
