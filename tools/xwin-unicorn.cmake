# CMake toolchain for unicorn-engine-sys under `cargo xwin` (i686 MSVC;
# .cargo/config.toml points CMAKE_TOOLCHAIN_FILE_i686-pc-windows-msvc here,
# which the cmake crate prefers to the underscore form cargo-xwin sets).
#
# cargo-xwin's own toolchain file sets CMAKE_MSVC_RUNTIME_LIBRARY, and the
# cmake crate also passes -MT in CMAKE_C_FLAGS; unicorn's CMakeLists.txt
# refuses both at once ("please set the runtime library via either
# CMAKE_C_FLAGS or CMAKE_MSVC_RUNTIME_LIBRARY, not both"). This includes
# cargo-xwin's file and drops the variable, so -MT (the static CRT that
# dinput8.dll links) decides.
include("$ENV{CMAKE_TOOLCHAIN_FILE_i686_pc_windows_msvc}")
unset(CMAKE_MSVC_RUNTIME_LIBRARY CACHE)
unset(CMAKE_MSVC_RUNTIME_LIBRARY)
