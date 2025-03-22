#include "SingletonAccessor.hpp"
#include "core/print_string.hpp"
#include "godot_cpp/classes/scene_tree.hpp"
#include <cstdio>
#include <godot_cpp/classes/window.hpp>

using namespace std;
using namespace godot;

Node *SingletonAccessor::get_singleton(const String &singleton_name) {
    SceneTree *tree = Object::cast_to<SceneTree>(Engine::get_singleton()->get_main_loop());
    if (!tree) {
        godot::print_line("❌ SceneTree is NOT available!");
        return nullptr;
    }

    godot::print_line("SceneTree: ", tree);

    // 🔹 Get Root as Window (Godot 4.3 returns Window*)
    Window *root_window = Object::cast_to<Window>(tree->get_root());
    if (!root_window) {
        godot::print_line("❌ Root Window not found!");
        return nullptr;
    }

    // 🔹 Try casting Window to Node
    Node *root = Object::cast_to<Node>(root_window);
    if (!root) {
        godot::print_line("❌ Failed to cast Window to Node!");
        return nullptr;
    }

    godot::print_line("root node: ", root);

    // 🔹 Print Root Node's Children
    Array children = root->get_children();
    godot::print_line("🔍 Checking Root Node's Children:");
    for (int i = 0; i < children.size(); i++) {
        Node *child = Object::cast_to<Node>(children[i]);
        if (child) {
            godot::print_line("📌 Found Child Node: " + child->get_name());
        }
    }

    // 🔹 Lookup Singleton
    String full_path = "/root/" + singleton_name;
    Node *singleton_node = root->get_node<Node>(NodePath(full_path));

    if (!singleton_node) {
        godot::print_line("❌ Singleton '" + singleton_name + "' not found!");
    } else {
        godot::print_line("✅ Singleton Found: " + singleton_name);
    }

    godot::print_line(singleton_name , " found at path: " , full_path);
    return singleton_node;
}

