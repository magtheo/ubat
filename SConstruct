import os

print("Running SConstruct in:", Dir(".").abspath)

# Create the build environment.
env = Environment(tools=["default"])
env.Append(CPPFLAGS=["-std=c++17"])

# Initialize CPPPATH if not present.
if "CPPPATH" not in env:
    env["CPPPATH"] = []

# Append include directories.
env.Append(CPPPATH=[
    "godot-cpp/gdextension",  # For gdextension_interface.h (if needed)
    "godot-cpp/include",      # For headers like godot_cpp/core/class_db.hpp
    "godot-cpp/gen/include"   # For generated headers
])

# Append library path and libraries.
env.Append(LIBPATH=["godot-cpp/bin"])
env.Append(LIBS=["godot-cpp.linux.template_release.x86_64"])

# Debug: print out the CPPPATH.
print("CPPPATH list after append:", env["CPPPATH"])
print("Substituted CPPPATH:", env.subst("$CPPPATH"))

# Build shared library.
target = "bin/libterrain_generator.so" if env["PLATFORM"] == "linux" else "bin/terrain_generator.dll"
sources = Glob("src/*.cpp")

env.SharedLibrary(target, sources)
