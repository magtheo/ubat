#include "SingletonAccessor.hpp"
#include <cstdio>
#include <godot_cpp/classes/window.hpp>
#include <iostream>
using namespace std;
using namespace godot;

Node *SingletonAccessor::get_singleton(const String &singleton_name) {
    // Get the main loop, which should be the SceneTree.
    SceneTree *tree = Object::cast_to<SceneTree>(Engine::get_singleton()->get_main_loop());
    if (!tree) {
        printf("SingletonAccessor: SceneTree is not available!\n");
        return nullptr;
    }
    
    // Get the root node and cast it to Node (since get_root() returns Window* in Godot 4.3).
    Node *root = Object::cast_to<Node>(tree->get_root());
    if (!root) {
        printf("SingletonAccessor: Root node not found!\n");
        return nullptr;
    }
    
    // Retrieve the singleton node by its autoload name.
    Node *singleton_node = root->get_node<Node>(NodePath(singleton_name));
    if (!singleton_node) {
        printf("SingletonAccessor: Singleton node '%s' not found!\n", singleton_name.utf8().get_data());
    }
    cout << "singleton_node";
    return singleton_node;
}
