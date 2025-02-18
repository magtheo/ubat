#include "SingletonAccessor.hpp"

using namespace godot;

Node *SingletonAccessor::get_singleton(const String &singleton_name) {
    // Get the main loop, which should be the SceneTree.
    SceneTree *tree = Object::cast_to<SceneTree>(Engine::get_singleton()->get_main_loop());
    if (!tree) {
        printf("SingletonAccessor: SceneTree is not available!");
        return nullptr;
    }
    
    // Get the root node of the scene.
    Node *root = tree->get_root(); // TODO: resolve errors
    if (!root) {
        printf("SingletonAccessor: Root node not found!");
        return nullptr;
    }
    
    // Retrieve the singleton node by its autoload name.
    Node *singleton_node = root->get_node(singleton_name);
    if (!singleton_node) {
        printf(String("SingletonAccessor: Singleton node '") + singleton_name + "' not found!");
    }
    
    return singleton_node;
}
