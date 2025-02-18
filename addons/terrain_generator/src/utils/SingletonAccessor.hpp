#ifndef SINGLETON_ACCESSOR_HPP
#define SINGLETON_ACCESSOR_HPP

#include <godot_cpp/classes/node.hpp>
#include <godot_cpp/classes/scene_tree.hpp>
#include <godot_cpp/classes/engine.hpp>
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/variant/string.hpp>

namespace godot {

class SingletonAccessor {
public:
    /// Returns a pointer to the autoload singleton node with the given name.
    /// If not found, it returns nullptr.
    static Node *get_singleton(const String &singleton_name);
};

} // namespace godot

#endif // SINGLETON_ACCESSOR_HPP
