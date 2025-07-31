.PHONY: all clean build-go build-rust test

# 检测操作系统
OS := $(shell uname -s)

ifeq ($(OS),Linux)
    BUILD_OS := linux
    DLL_EXT := so
    DLL_PREFIX := lib
endif

ifeq ($(OS),Darwin)
    BUILD_OS := darwin
    DLL_EXT := dylib
    DLL_PREFIX := lib
endif

ifeq ($(findstring MINGW,$(OS)),MINGW)
    BUILD_OS := windows
    DLL_EXT := dll
    DLL_PREFIX := lib
endif

all: build-go build-rust

build-go:
    @echo "Building Go library for $(BUILD_OS)..."
    @mkdir -p build/$(BUILD_OS)
    @cd go && GOOS=$(BUILD_OS) go build -buildmode=c-shared -o ../build/$(BUILD_OS)/$(DLL_PREFIX)singbox.$(DLL_EXT) .
    @echo "Go library built successfully!"

build-rust:
    @echo "Building Rust library for $(BUILD_OS)..."
    @cd rust && cargo build --release
    @echo "Rust library built successfully!"

clean:
    @echo "Cleaning up..."
    @rm -rf build/*
    @cd rust && cargo clean
    @echo "Cleanup complete!"