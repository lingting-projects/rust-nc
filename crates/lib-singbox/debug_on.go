//go:build ncdebug

package core

// DebugEnabled 表示当前内核构建支持调试模式。
//
// 由构建标签 ncdebug 打开: Rust 侧仅在 debug 编译时传入该标签,
// 因此正式发布的内核里不存在调试分支。
const DebugEnabled = true
