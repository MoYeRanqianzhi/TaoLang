// ============================================================================
// LLVM 内存安全集成测试
// ============================================================================
// 验证 LLVMPrintModuleToFile 方式（避免 CRT 堆不匹配）能正确获取模块 IR。
// 此测试独立于 taoc 库，直接调用 llvm-sys API 进行验证。
// ============================================================================

use std::ffi::CString;
use std::ptr;

/// 测试：通过 LLVMPrintModuleToFile 安全获取模块 IR 字符串
///
/// 验证"写临时文件 + 读回"策略能正确获取 LLVM 模块的 IR 输出，
/// 且不触发任何 CRT 堆不匹配相关的崩溃。
#[test]
fn test_print_module_to_file_roundtrip() {
    unsafe {
        // 创建 LLVM 上下文和模块
        let ctx = llvm_sys::core::LLVMContextCreate();
        let name = CString::new("file_test").unwrap();
        let module = llvm_sys::core::LLVMModuleCreateWithNameInContext(name.as_ptr(), ctx);

        // 创建临时文件路径
        let temp_path = std::env::temp_dir().join("taoc_test_ir.ll");
        let c_path = CString::new(
            temp_path.to_str().expect("temp path should be valid UTF-8")
        ).unwrap();

        // 写入临时文件
        let mut error_msg: *mut i8 = ptr::null_mut();
        let result = llvm_sys::core::LLVMPrintModuleToFile(
            module,
            c_path.as_ptr(),
            &mut error_msg,
        );
        assert_eq!(result, 0, "LLVMPrintModuleToFile should succeed");

        // 读取文件内容
        let ir = std::fs::read_to_string(&temp_path)
            .expect("should be able to read temp file");

        // 清理临时文件
        let _ = std::fs::remove_file(&temp_path);

        // 验证内容
        assert!(ir.contains("file_test"), "IR should contain module name, got: {}", ir);

        // 清理 LLVM 资源
        llvm_sys::core::LLVMDisposeModule(module);
        llvm_sys::core::LLVMContextDispose(ctx);
    }
}

/// 测试：编译期目标三元组可用性
///
/// 验证 build.rs 通过 cargo:rustc-env 传递的 TAOC_TARGET_TRIPLE
/// 在编译期可用，且包含合理的目标平台信息。
#[test]
fn test_compile_time_target_triple() {
    // env!() 在编译期展开，若变量不存在则编译失败
    let triple = env!("TAOC_TARGET_TRIPLE");
    assert!(!triple.is_empty(), "TAOC_TARGET_TRIPLE should not be empty");
    assert!(
        triple.contains("windows") || triple.contains("linux") || triple.contains("darwin"),
        "triple should contain OS name, got: {}",
        triple
    );
}
