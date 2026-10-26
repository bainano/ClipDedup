fn main() {
    // 仅在 Windows 目标上把 assets/icon.ico 嵌入为 exe 图标。
    // 找不到资源编译器时会输出 cargo:warning，不影响构建。
    #[cfg(windows)]
    let _ = embed_resource::compile("assets/icon.rc", embed_resource::NONE);
}
