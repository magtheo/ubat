# CLAUDE.md - Guidance for Claude Code

## Build/Test Commands
- Build project: `cargo build --manifest-path addons/ubat/Cargo.toml`
- Run tests: `cargo test --manifest-path addons/ubat/Cargo.toml`
- Run single test: `cargo test --manifest-path addons/ubat/Cargo.toml <test_name>`
- Format code: `cargo fmt --manifest-path addons/ubat/Cargo.toml`
- Check code: `cargo clippy --manifest-path addons/ubat/Cargo.toml`

## Code Style Guidelines
- **Imports**: Group imports by std, external crates, and local modules
- **Naming**: Use snake_case for variables/functions, PascalCase for types/structs
- **Error Handling**: Use Result types with descriptive error messages; log errors with ErrorLogger
- **Formatting**: Follow Rust standard formatting (use cargo fmt)
- **Comments**: Use /// for public API docs, // for implementation notes
- **Error Logging**: Use the ErrorLogger with appropriate severity levels
- **Threading**: Use Arc/Mutex for thread-safe access patterns
- **Testing**: Create unit tests in a #[cfg(test)] module with descriptive test names

# Documents that will help in developing rust godot codebase
## Rust-Godot Technical Guidelines

### Core Architecture Principles

1. **Library Structure**
   - Maintain separate directories for Godot and Rust components
   - Use `crate-type = ["cdylib"]` in `Cargo.toml` to create a dynamic C library
   - Configure the `.gdextension` file to properly locate your compiled Rust library

2. **Memory Management**
   - Use `Gd<T>` smart pointer for all Godot object references
   - Be aware of different memory management models:
     - `RefCounted` objects are automatically freed when last reference is dropped
     - Objects created with `new_alloc()` require manual memory management (call `free()`)
   - Never attempt to access destroyed objects (gdext will panic safely)

3. **Class Design**
   - Define classes with `#[derive(GodotClass)]`
   - Specify base class with `#[class(base=BaseClass)]`
   - Include `base: Base<BaseClass>` field to access base functionality
   - Use `self.base()` and `self.base_mut()` to access base class methods
   - Avoid inheriting from user-defined classes; use composition instead

### API Integration

1. **Function Registration**
   - Mark all API-exposed impl blocks with `#[godot_api]`
   - Use `#[func]` to expose methods to Godot
   - Use `#[func(virtual)]` for script-override methods
   - Return `Gd<Self>` instead of `Self` for custom constructors
   - Prefer `Gd::from_init_fn()` or `Gd::from_object()` for constructing objects

2. **Property Exposure**
   - Use `#[var]` to expose fields to GDScript
   - Use `#[export]` to additionally make fields visible in the editor
   - For custom types used as properties, implement `GodotConvert`, `Var` and `Export` traits
   - For enums, specify representation with `#[godot(via = GString)]` or `#[godot(via = i64)]`

3. **Object Access Patterns**
   - Access Rust object through a `Gd<T>` pointer using:
     - `gd.bind()` for immutable access
     - `gd.bind_mut()` for mutable access
   - Don't combine `bind()/bind_mut()` with `base()/base_mut()`
   - Use `to_gd()` method to obtain a `Gd<Self>` pointer from within an object

4. **Type Conversions**
   - Convert between string types with `.arg()` when passing as arguments
   - Use `upcast::<BaseClass>()` for infallible upcasts
   - Use `cast::<DerivedClass>()` for confident downcasts, or `try_cast::<DerivedClass>()` if fallible
   - Convert Rust types to Godot Variant with `.to_variant()`

### Performance Considerations

1. **Array and Dictionary Usage**
   - Use `Array<T>` for type-safe collections, `VariantArray` for dynamically typed ones
   - Be mindful that `Array` and `Dictionary` use reference counting, not copy-on-write
   - Use `PackedArray` types for optimized memory usage, but be aware of copy-on-write semantics
   - Don't modify collections during iteration

2. **String Handling**
   - Use appropriate string type: `GString` for general text, `StringName` for identifiers
   - Prefer C-string literals (`c"string"`) for static `StringName` values when possible
   - Be aware that string conversions can be expensive due to UTF-8/UTF-32 encoding differences

3. **Argument Passing**
   - Pass references (`&T`) to non-Copy types when possible
   - Use `.arg()` explicitly for string type conversions

### Compatibility and Deployment

1. **Version Selection**
   - Target lower API versions for wider compatibility:
     - Use `features = ["api-4-2"]` to target Godot 4.2
     - Runtime Godot version must be >= API version
   - Do not use Godot 4.0 (lacks compatibility even among its own patch versions)

2. **Platform-Specific Considerations**
   - Use universal builds for macOS
   - Configure thread support correctly for web exports
   - Set up cross-compilation correctly for mobile targets
   - Use the `.gdextension` file to specify platform-specific paths to your compiled libraries

3. **Editor Integration**
   - Use `#[class(tool)]` for resources or plugins that need to run in the editor
   - Register custom icons in the `.gdextension` file
   - Consider performance implications when running code in editor context

By following these guidelines, you'll create Rust extensions for Godot that are safe, performant, and follow the library's best practices.


# Rust-Godot Borrowing Patterns and Solutions

When working with Godot from Rust, you'll frequently encounter borrowing issues due to the intersection of Rust's ownership system and Godot's object model. This document outlines common patterns and solutions.

## The "Gather Data, Then Modify" Pattern

### Problem:
Simultaneous immutable and mutable borrows of `self` or its fields:

```rust
fn update_something(&mut self) {
    // Immutable borrow through binding
    let data_ref = self.some_field.bind();
    
    // ERROR: Can't borrow self mutably while an immutable borrow is active
    self.other_field.bind_mut().modify_something();
}
```

### Solution:
Gather all data from immutable borrows first, then perform mutable operations:

```rust
fn update_something(&mut self) {
    // Step 1: Gather all data using immutable borrows
    let data = if let Some(field) = &self.some_field {
        let result = field.bind().get_some_data();
        if result.is_empty() {
            return;
        }
        result
    } else {
        return;
    };
    
    // Step 2: Immutable borrows are now dropped, safe to do mutable operations
    if let Some(other) = &self.other_field {
        let mut mutable_ref = other.clone();
        mutable_ref.bind_mut().modify_something(data);
    }
}
```

### Benefits:
- Resolves borrowing conflicts
- Makes data flow more explicit
- Negligible performance impact

## Working with `Gd<T>` and Node References

### Problem:
Borrowing conflicts when referencing Godot objects and modifying them:

```rust
// ERROR: Can't have mut and immut borrows simultaneously
self.base_mut().add_child(self.some_node.bind().get_something().upcast());
```

### Solution:
Clone objects, perform operations on the clone, and be mindful of method order:

```rust
// Clone the node reference to avoid borrowing conflicts
let node_clone = self.some_node.clone();
let data = node_clone.bind().get_something();

// Now use the data with base_mut()
self.base_mut().process_data(data);
```

### For adding children to nodes:
```rust
// 1. Create the node
let mut new_node = SomeNode::new_alloc();

// 2. Configure it
new_node.set_property(value);

// 3. Upcast to Node type first, then get reference
let node_ref = new_node.clone().upcast::<Node>();
self.base_mut().add_child(&node_ref);

// 4. Store for later use
self.my_nodes.insert(key, new_node);
```

## Managing Mutable Access with HashMap

### Problem:
Borrowing conflicts when using HashMap and trying to modify values:

```rust
// ERROR: Cannot borrow as mutable
if let Some(obj) = self.objects.get(&key) {
    obj.some_method(); // Error if method requires mut
}
```

### Solutions:

1. **Clone and modify approach**:
```rust
if let Some(obj) = self.objects.get(&key) {
    let mut obj_clone = obj.clone();
    obj_clone.bind_mut().some_method();
}
```

2. **Get and insert approach** (for replacing values):
```rust
if let Some(obj) = self.objects.get(&key) {
    let mut obj_clone = obj.clone();
    obj_clone.bind_mut().some_method();
    // Replace the old value
    self.objects.insert(key, obj_clone);
}
```

3. **Use entry API** (for complex updates):
```rust
self.objects.entry(key)
    .and_modify(|obj| {
        let mut obj_clone = obj.clone();
        obj_clone.bind_mut().some_method();
        *obj = obj_clone;
    });
```

## Handling Object Lifecycle

### Problem:
Need to free Godot objects manually while dealing with borrowing rules:

```rust
// ERROR: Cannot borrow as mutable
for (_, obj) in self.objects.drain() {
    obj.queue_free(); // Needs mut
}
```

### Solution:
Mark the loop variable as mutable:

```rust
// Correct: with mut keyword
for (_, mut obj) in self.objects.drain() {
    obj.queue_free();
}
```

## Type Specification with `upcast<T>()`

### Problem:
Ambiguous hierarchy when upcasting:

```rust
// ERROR: Cannot determine which trait impl to use
node.upcast().do_something();
```

### Solution:
Explicitly specify the target type:

```rust
// Specify the target type for upcast
node.upcast::<Node>().do_something();
```

## Performance Considerations

- These borrowing patterns have negligible performance impact
- The "gather data, then modify" approach doesn't cause any significant overhead
- Cloning `Gd<T>` pointers is cheap (they're reference-counted)
- The compiler eliminates most of these abstractions during optimization

## When All Else Fails

If you're dealing with extremely complex borrowing situations:

1. **Refactor into smaller methods** - Split complex functions into smaller ones with clearer ownership
2. **Use temporary variables** - Store intermediate results to reduce nested borrows
3. **Consider interior mutability** - For rare cases, `RefCell` can help (though use sparingly)
4. **Restructure your data model** - Sometimes the best solution is to reconsider how data is organized


# Advanced Signal Emission and Configuration Management in Godot-Rust

## Signal Emission and Shared State Challenges

### The Core Problem: Borrowing Conflicts in Godot-Rust

When developing Godot extensions in Rust, developers often encounter complex borrowing scenarios that challenge the language's strict ownership rules. This section explores a sophisticated pattern for managing signal emissions and configuration updates while respecting Rust's borrowing constraints.

#### Common Borrowing Antipatterns

Consider the following problematic code snippet:

```rust
// Problematic signal emission
fn update_config(&self, key: String, value: Variant) -> bool {
    // Immutable lock on configuration
    let config_manager = self.config_manager.lock().unwrap();
    
    // PROBLEM: Cannot emit signal due to active immutable borrow
    self.base_mut().emit_signal("config_updated", &[key, value]);
    
    true
}
```

This code fails because:
1. The mutex lock creates an immutable borrow
2. Signal emission requires a mutable reference
3. Rust's borrow checker prevents simultaneous mutable and immutable borrows

### Refined Configuration Management Pattern

#### Key Design Principles

1. **Separate Result Computation**: Calculate configuration updates first
2. **Minimize Lock Duration**: Release mutex locks quickly
3. **Signal Emission After State Management**: Emit signals after all state modifications
4. **Explicit Error Handling**: Provide clear error paths

#### Implementation Strategy

```rust
fn sync_property_to_config(&mut self, property_name: &str, value: Variant) -> bool {
    // Compute result in a separate, controlled scope
    let result = if let Some(config_manager) = &self.config_manager {
        config_manager.lock().map(|mut manager| {
            // Clone configuration to avoid direct mutation
            let mut config = manager.get_config().clone();
            
            // Perform configuration updates
            match property_name {
                "world_seed" => {
                    config.world_seed = self.world_seed as u64;
                },
                "network_mode" => {
                    // Complex mode conversion logic
                    config.game_mode = match self.network_mode {
                        0 => GameModeConfig::Standalone,
                        1 => GameModeConfig::Host(HostConfig {
                            world_generation_seed: config.world_seed,
                            admin_password: None,
                        }),
                        2 => GameModeConfig::Client(ClientConfig {
                            server_address: self.server_address.to_string(),
                            username: "Player".to_string(),
                        }),
                        _ => return false, // Explicit error handling
                    };
                },
                // Additional property mappings...
                _ => return false,
            }
            
            // Update configuration
            manager.update_config(config);
            true
        }).unwrap_or(false)
    } else {
        false
    };

    // Signal emission AFTER lock release
    if result {
        self.base_mut().emit_signal(
            &StringName::from("config_updated"), 
            &[property_name.to_variant(), value]
        );
    }

    result
}
```

### Deep Dive: Why This Pattern Works

#### Ownership and Borrowing Explained

Rust's ownership system prevents data races by enforcing strict rules:
- Only one mutable reference to data at a time
- Multiple immutable references are allowed
- References must not outlive the data they reference

This pattern resolves conflicts by:
1. Cloning configuration before mutation
2. Releasing locks before signal emission
3. Separating computation from side effects

### Performance Considerations

While cloning might seem expensive, modern Rust optimizations make this approach efficient:
- Configuration structs are typically small
- Clone operations are often optimized by the compiler
- Prevents complex lifetime management

### Error Handling and Resilience

The implementation provides multiple layers of error handling:
- Early returns for invalid configurations
- Mutex lock failure handling
- Explicit boolean result communication

### Learning Objectives

After studying this pattern, developers should understand:
- How to manage shared, mutable state in Rust
- Strategies for signal emission in Godot extensions
- Advanced ownership and borrowing techniques

### Recommended Practices

1. Minimize lock duration
2. Clone state when mutation is complex
3. Separate computation from side effects
4. Use explicit error handling
5. Prefer immutability when possible

### Reflection Questions

- How does this pattern differ from traditional object-oriented approaches?
- What are the trade-offs between safety and performance?
- How might you adapt this pattern to more complex state management scenarios?

## Conclusion

The presented pattern represents a sophisticated approach to managing configuration and signal emission in Godot-Rust extensions. By understanding and applying these principles, developers can create robust, safe, and performant game extensions that leverage Rust's powerful type system and ownership model.
