// ============================================================================
// llvm_wrapper.rs — LLVM C API 安全封装层
// ============================================================================
// 将 llvm-sys 提供的原始 C FFI 调用封装为安全的 Rust 类型。
// 所有 unsafe 代码集中在此文件中，其他模块通过安全接口使用 LLVM 功能。
//
// Clippy: LLVM 的 LLVMTypeRef、LLVMValueRef、LLVMBasicBlockRef 等类型是
// 原始指针（*mut LLVMOpaqueType 等），本模块的公开函数接受这些指针作为参数，
// 并在 unsafe 块中传递给 LLVM C API。这些指针来源于其他 LLVM API 调用，
// LLVM 保证其有效性，不存在悬垂指针风险。因此全局允许 not_unsafe_ptr_arg_deref。
#![allow(clippy::not_unsafe_ptr_arg_deref)]
//
// 封装的核心类型：
//   - LlvmContext: LLVM 上下文（拥有所有类型和常量）
//   - LlvmModule:  LLVM 模块（一个编译单元）
//   - LlvmBuilder: LLVM IR 构建器（构造指令）
//
// 内存安全策略（Windows CRT 堆不匹配问题）：
//   LLVM 预编译静态库使用 /MT（静态 CRT），Rust 使用 /MD（动态 CRT）。
//   链接器在最终二进制中可能将 malloc/free 解析到不同 CRT 实现，
//   导致 LLVMDisposeMessage(free) 释放 strdup(malloc) 分配的内存时崩溃。
//   C 桥接方案无效——因为 free() 的解析发生在链接器层面，与调用来源无关。
//
//   最终解决方案：彻底避免调用任何返回 malloc'd 字符串的 LLVM API：
//     1. LLVMPrintModuleToString → 改用 LLVMPrintModuleToFile 写临时文件再读回
//     2. LLVMGetDefaultTargetTriple → 改用 build.rs 传递的编译期目标三元组
//     3. LLVMSetModuleDataLayout → 模块接管 TargetData 所有权，无需手动释放
//     4. 错误消息（LLVMGetTargetFromTriple 等）→ C 桥接层尽力释放（仅错误路径）
//   详见 docs/problems/crt-mismatch.md
// ============================================================================

use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};

use llvm_sys::core::*;
use llvm_sys::prelude::*;
use llvm_sys::target::*;
use llvm_sys::target_machine::*;

use crate::error::TaoError;

/// 全局原子计数器，确保并发调用 print_to_string 时临时文件名唯一
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

// ============================================================================
// LLVM 错误消息释放桥接函数（C 侧实现，见 csrc/llvm_string_bridge.c）
// ============================================================================
// 仅用于释放 LLVM 错误消息（LLVMGetTargetFromTriple、
// LLVMTargetMachineEmitToFile 等失败时返回的错误字符串）。
// 注意：由于 CRT 堆不匹配，此释放可能不安全，但仅在编译器报错退出前调用，
// 即使崩溃也不影响正常编译流程。
// ============================================================================
unsafe extern "C" {
    /// 安全释放 LLVM 错误消息字符串
    fn taoc_dispose_llvm_message(msg: *mut i8);
}

/// LLVM 上下文的安全封装
///
/// LLVM 上下文拥有与其关联的所有类型、常量和其他 IR 对象的所有权。
/// 通过 Drop 自动释放。
pub struct LlvmContext {
    /// 原始 LLVM 上下文指针
    raw: LLVMContextRef,
}

impl LlvmContext {
    /// 创建一个新的 LLVM 上下文
    pub fn new() -> Self {
        let raw = unsafe { LLVMContextCreate() };
        Self { raw }
    }

    /// 获取原始指针（供需要直接操作 LLVM API 的场景使用）
    pub fn as_raw(&self) -> LLVMContextRef {
        self.raw
    }

    /// 在此上下文中获取 i32 类型
    pub fn i32_type(&self) -> LLVMTypeRef {
        unsafe { LLVMInt32TypeInContext(self.raw) }
    }

    /// 在此上下文中获取 i8 类型
    pub fn i8_type(&self) -> LLVMTypeRef {
        unsafe { LLVMInt8TypeInContext(self.raw) }
    }

    /// 在此上下文中获取 opaque pointer 类型（LLVM 21 使用 opaque pointer）
    pub fn ptr_type(&self) -> LLVMTypeRef {
        unsafe { LLVMPointerTypeInContext(self.raw, 0) }
    }

    /// 在此上下文中获取 void 类型
    pub fn void_type(&self) -> LLVMTypeRef {
        unsafe { LLVMVoidTypeInContext(self.raw) }
    }
}

impl Default for LlvmContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LlvmContext {
    fn drop(&mut self) {
        unsafe { LLVMContextDispose(self.raw) };
    }
}

/// LLVM 模块的安全封装
///
/// 一个 LLVM 模块对应一个编译单元，包含函数定义、全局变量等。
/// 注意：模块的生命周期必须短于创建它的上下文。
pub struct LlvmModule {
    /// 原始 LLVM 模块指针
    raw: LLVMModuleRef,
}

impl LlvmModule {
    /// 在指定上下文中创建一个新的 LLVM 模块
    ///
    /// # 参数
    /// - `name`: 模块名称（通常为源文件名）
    /// - `context`: LLVM 上下文
    pub fn new(name: &str, context: &LlvmContext) -> Self {
        let c_name = CString::new(name).expect("module name should not contain null bytes");
        let raw = unsafe {
            LLVMModuleCreateWithNameInContext(c_name.as_ptr(), context.as_raw())
        };
        Self { raw }
    }

    /// 获取原始指针
    pub fn as_raw(&self) -> LLVMModuleRef {
        self.raw
    }

    /// 设置模块的目标三元组
    pub fn set_target_triple(&self, triple: &str) {
        let c_triple = CString::new(triple).unwrap();
        unsafe { LLVMSetTarget(self.raw, c_triple.as_ptr()) };
    }

    /// 设置模块的数据布局
    pub fn set_data_layout(&self, layout: &str) {
        let c_layout = CString::new(layout).unwrap();
        unsafe { LLVMSetDataLayout(self.raw, c_layout.as_ptr()) };
    }

    /// 向模块添加函数声明或定义
    ///
    /// 如果函数已存在，返回已有的函数引用。
    pub fn add_function(&self, name: &str, fn_type: LLVMTypeRef) -> LLVMValueRef {
        let c_name = CString::new(name).unwrap();
        unsafe { LLVMAddFunction(self.raw, c_name.as_ptr(), fn_type) }
    }

    /// 根据名称查找模块中的函数
    pub fn get_function(&self, name: &str) -> Option<LLVMValueRef> {
        let c_name = CString::new(name).unwrap();
        let func = unsafe { LLVMGetNamedFunction(self.raw, c_name.as_ptr()) };
        if func.is_null() {
            None
        } else {
            Some(func)
        }
    }

    /// 将模块 IR 输出为字符串（用于调试和测试）
    ///
    /// 通过 LLVMPrintModuleToFile 将 IR 写入临时文件，再读回 Rust String。
    /// 此方法完全避免调用 LLVMPrintModuleToString（返回 malloc'd 字符串），
    /// 从而绕过 Windows CRT 堆不匹配导致的 LLVMDisposeMessage 崩溃问题。
    /// LLVM 内部的文件写入使用 stdio（fopen/fprintf/fclose），不涉及跨堆操作。
    pub fn print_to_string(&self) -> String {
        // 创建唯一临时文件路径（PID + 原子计数器，确保多线程安全）
        let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_path = std::env::temp_dir().join(format!(
            "taoc_ir_{}_{}.ll",
            std::process::id(),
            counter,
        ));
        let c_path = CString::new(
            temp_path.to_str().expect("temp path should be valid UTF-8")
        ).expect("temp path should not contain null bytes");

        unsafe {
            let mut error_msg: *mut i8 = ptr::null_mut();
            let result = LLVMPrintModuleToFile(
                self.raw,
                c_path.as_ptr(),
                &mut error_msg,
            );
            if result != 0 {
                // LLVMPrintModuleToFile 失败（极其罕见）
                let msg = if !error_msg.is_null() {
                    let s = CStr::from_ptr(error_msg).to_string_lossy().into_owned();
                    // 尽力释放错误消息（仅错误路径）
                    taoc_dispose_llvm_message(error_msg);
                    s
                } else {
                    "unknown error".into()
                };
                // 清理可能残留的临时文件
                let _ = std::fs::remove_file(&temp_path);
                panic!("LLVMPrintModuleToFile failed: {}", msg);
            }
        }

        // 读取临时文件内容并删除
        let ir = std::fs::read_to_string(&temp_path)
            .expect("should be able to read IR temp file");
        let _ = std::fs::remove_file(&temp_path);
        ir
    }

    /// 初始化 LLVM 目标后端并设置模块的 target triple 和 data layout
    ///
    /// 将 LLVM 目标初始化逻辑从 `emit_object_file` 中抽出，使 `--emit-ir` 模式
    /// 也能在 IR 中包含正确的 target triple 和 target datalayout 信息。
    ///
    /// 目标三元组使用编译期常量 TAOC_TARGET_TRIPLE（由 build.rs 从 Cargo
    /// 的 TARGET 环境变量传递），完全避免调用 LLVMGetDefaultTargetTriple
    /// （返回 malloc'd 字符串，存在 CRT 堆不匹配问题）。
    pub fn set_target_metadata(&self) -> Result<(), TaoError> {
        unsafe {
            // 初始化所有 LLVM 目标后端
            LLVM_InitializeAllTargetInfos();
            LLVM_InitializeAllTargets();
            LLVM_InitializeAllTargetMCs();
            LLVM_InitializeAllAsmParsers();
            LLVM_InitializeAllAsmPrinters();

            // 使用编译期目标三元组（build.rs 通过 cargo:rustc-env 传递）
            // 完全避免运行时调用 LLVMGetDefaultTargetTriple（CRT 堆不匹配）
            let target_triple = CString::new(env!("TAOC_TARGET_TRIPLE"))
                .expect("target triple should not contain null bytes");
            LLVMSetTarget(self.raw, target_triple.as_ptr());

            // 查找目标（用于获取数据布局）
            let mut target: LLVMTargetRef = ptr::null_mut();
            let mut error_msg: *mut i8 = ptr::null_mut();
            let result = LLVMGetTargetFromTriple(
                target_triple.as_ptr(),
                &mut target,
                &mut error_msg,
            );
            if result != 0 {
                let msg = if !error_msg.is_null() {
                    let s = CStr::from_ptr(error_msg).to_string_lossy().into_owned();
                    // 通过桥接层尽力释放 LLVM 错误消息（仅错误路径）
                    taoc_dispose_llvm_message(error_msg);
                    s
                } else {
                    "unknown error".into()
                };
                return Err(TaoError::CodegenError {
                    message: format!("failed to get target: {}", msg),
                });
            }

            // 创建目标机器以获取数据布局（O0 优化级别）
            let cpu = CString::new("generic").unwrap();
            let features = CString::new("").unwrap();
            let target_machine = LLVMCreateTargetMachine(
                target,
                target_triple.as_ptr(),
                cpu.as_ptr(),
                features.as_ptr(),
                LLVMCodeGenOptLevel::LLVMCodeGenLevelNone,  // O0 无优化
                LLVMRelocMode::LLVMRelocDefault,
                LLVMCodeModel::LLVMCodeModelDefault,
            );

            if target_machine.is_null() {
                return Err(TaoError::CodegenError {
                    message: "failed to create target machine".into(),
                });
            }

            // 设置模块数据布局
            // LLVMSetModuleDataLayout 后模块接管 data_layout 的所有权，
            // 无需手动调用 LLVMDisposeTargetData（否则双重释放）。
            let data_layout = LLVMCreateTargetDataLayout(target_machine);
            LLVMSetModuleDataLayout(self.raw, data_layout);

            // 清理目标机器资源（data_layout 已移交给模块）
            LLVMDisposeTargetMachine(target_machine);

            Ok(())
        }
    }

    /// 将模块 IR 直接写入指定文件路径
    ///
    /// 使用 `LLVMPrintModuleToFile` 直接写入用户指定路径，
    /// 避免 `print_to_string` 的"写临时文件→读回→再写"多余往返。
    /// LLVM 内部的文件写入使用 stdio（fopen/fprintf/fclose），不涉及跨堆操作。
    pub fn emit_ir_to_file(&self, output_path: &Path) -> Result<(), TaoError> {
        let c_path = CString::new(
            output_path.to_str().expect("output path should be valid UTF-8")
        ).expect("output path should not contain null bytes");

        unsafe {
            let mut error_msg: *mut i8 = ptr::null_mut();
            let result = LLVMPrintModuleToFile(
                self.raw,
                c_path.as_ptr(),
                &mut error_msg,
            );
            if result != 0 {
                let msg = if !error_msg.is_null() {
                    let s = CStr::from_ptr(error_msg).to_string_lossy().into_owned();
                    // 尽力释放错误消息（仅错误路径）
                    taoc_dispose_llvm_message(error_msg);
                    s
                } else {
                    "unknown error".into()
                };
                return Err(TaoError::CodegenError {
                    message: format!("failed to write IR file: {}", msg),
                });
            }
        }

        Ok(())
    }

    /// 将模块编译为目标文件（.obj）
    ///
    /// 先调用 `set_target_metadata` 设置目标信息，再发射目标文件。
    pub fn emit_object_file(&self, output_path: &Path) -> Result<(), TaoError> {
        // 确保目标元数据已设置（target triple + data layout）
        self.set_target_metadata()?;

        unsafe {
            // 输出目标文件
            // 使用编译期目标三元组获取目标机器用于代码发射
            let target_triple = CString::new(env!("TAOC_TARGET_TRIPLE"))
                .expect("target triple should not contain null bytes");

            // 查找目标（set_target_metadata 已初始化后端，此处直接查找）
            let mut target: LLVMTargetRef = ptr::null_mut();
            let mut error_msg: *mut i8 = ptr::null_mut();
            let result = LLVMGetTargetFromTriple(
                target_triple.as_ptr(),
                &mut target,
                &mut error_msg,
            );
            if result != 0 {
                let msg = if !error_msg.is_null() {
                    let s = CStr::from_ptr(error_msg).to_string_lossy().into_owned();
                    taoc_dispose_llvm_message(error_msg);
                    s
                } else {
                    "unknown error".into()
                };
                return Err(TaoError::CodegenError {
                    message: format!("failed to get target: {}", msg),
                });
            }

            // 创建目标机器用于代码发射
            let cpu = CString::new("generic").unwrap();
            let features = CString::new("").unwrap();
            let target_machine = LLVMCreateTargetMachine(
                target,
                target_triple.as_ptr(),
                cpu.as_ptr(),
                features.as_ptr(),
                LLVMCodeGenOptLevel::LLVMCodeGenLevelNone,  // O0 无优化
                LLVMRelocMode::LLVMRelocDefault,
                LLVMCodeModel::LLVMCodeModelDefault,
            );

            if target_machine.is_null() {
                return Err(TaoError::CodegenError {
                    message: "failed to create target machine".into(),
                });
            }

            // 发射目标文件
            let output_c = CString::new(
                output_path.to_str().expect("output path should be valid UTF-8")
            ).unwrap();
            let mut error_msg: *mut i8 = ptr::null_mut();
            let result = LLVMTargetMachineEmitToFile(
                target_machine,
                self.raw,
                output_c.as_ptr() as *mut i8,
                LLVMCodeGenFileType::LLVMObjectFile,
                &mut error_msg,
            );

            // 清理目标机器资源
            LLVMDisposeTargetMachine(target_machine);

            if result != 0 {
                let msg = if !error_msg.is_null() {
                    let s = CStr::from_ptr(error_msg).to_string_lossy().into_owned();
                    // 通过桥接层尽力释放 LLVM 错误消息（仅错误路径）
                    taoc_dispose_llvm_message(error_msg);
                    s
                } else {
                    "unknown error".into()
                };
                return Err(TaoError::CodegenError {
                    message: format!("failed to emit object file: {}", msg),
                });
            }

            Ok(())
        }
    }
}

impl Drop for LlvmModule {
    fn drop(&mut self) {
        unsafe { LLVMDisposeModule(self.raw) };
    }
}

/// LLVM IR 构建器的安全封装
///
/// 构建器用于在基本块中逐条构造 LLVM IR 指令。
/// 使用前必须通过 `position_at_end` 定位到目标基本块。
pub struct LlvmBuilder {
    /// 原始 LLVM 构建器指针
    raw: LLVMBuilderRef,
}

impl LlvmBuilder {
    /// 在指定上下文中创建一个新的 IR 构建器
    pub fn new(context: &LlvmContext) -> Self {
        let raw = unsafe { LLVMCreateBuilderInContext(context.as_raw()) };
        Self { raw }
    }

    /// 获取原始指针
    pub fn as_raw(&self) -> LLVMBuilderRef {
        self.raw
    }

    /// 将构建器定位到指定基本块的末尾
    pub fn position_at_end(&self, block: LLVMBasicBlockRef) {
        unsafe { LLVMPositionBuilderAtEnd(self.raw, block) };
    }

    /// 构建函数调用指令
    ///
    /// # 参数
    /// - `fn_type`: 被调用函数的类型
    /// - `func`: 被调用的函数值
    /// - `args`: 参数值数组
    /// - `name`: 返回值的名称（void 函数可为空）
    pub fn build_call(
        &self,
        fn_type: LLVMTypeRef,
        func: LLVMValueRef,
        args: &mut [LLVMValueRef],
        name: &str,
    ) -> LLVMValueRef {
        let c_name = CString::new(name).unwrap();
        unsafe {
            LLVMBuildCall2(
                self.raw,
                fn_type,
                func,
                args.as_mut_ptr(),
                args.len() as u32,
                c_name.as_ptr(),
            )
        }
    }

    /// 构建 ret 指令（返回一个值）
    pub fn build_ret(&self, value: LLVMValueRef) -> LLVMValueRef {
        unsafe { LLVMBuildRet(self.raw, value) }
    }

    /// 构建 ret void 指令（无返回值）
    pub fn build_ret_void(&self) -> LLVMValueRef {
        unsafe { LLVMBuildRetVoid(self.raw) }
    }

    /// 获取构建器当前所在的基本块
    pub fn get_insert_block(&self) -> LLVMBasicBlockRef {
        unsafe { LLVMGetInsertBlock(self.raw) }
    }

    /// 构建全局字符串常量
    ///
    /// 在模块中创建全局字符串常量，返回指向字符串首字节的指针（ptr 类型）。
    /// LLVMBuildGlobalStringPtr 在 LLVM 21 中标记弃用但功能正常且稳定，
    /// 其替代 LLVMBuildGlobalString 返回的是数组类型而非指针，语义不同。
    ///
    /// # 错误
    /// 如果字符串包含 null 字节（\0），返回错误。C 字符串以 null 终止，
    /// 无法表示含嵌入 null 的字符串。
    #[allow(deprecated)]
    pub fn build_global_string_ptr(
        &self,
        value: &str,
        name: &str,
    ) -> Result<LLVMValueRef, String> {
        let c_value = CString::new(value).map_err(|e| {
            format!(
                "string literal contains null byte at position {} — \
                 C strings cannot contain embedded null bytes",
                e.nul_position()
            )
        })?;
        let c_name = CString::new(name).unwrap();
        Ok(unsafe {
            LLVMBuildGlobalStringPtr(self.raw, c_value.as_ptr(), c_name.as_ptr())
        })
    }
}

impl Drop for LlvmBuilder {
    fn drop(&mut self) {
        unsafe { LLVMDisposeBuilder(self.raw) };
    }
}

/// 在 LLVM 函数中追加一个新的基本块
///
/// # 参数
/// - `context`: LLVM 上下文
/// - `function`: 目标函数
/// - `name`: 基本块名称
pub fn append_basic_block(
    context: &LlvmContext,
    function: LLVMValueRef,
    name: &str,
) -> LLVMBasicBlockRef {
    let c_name = CString::new(name).unwrap();
    unsafe {
        LLVMAppendBasicBlockInContext(context.as_raw(), function, c_name.as_ptr())
    }
}

/// 创建函数类型
///
/// # 参数
/// - `return_type`: 返回值类型
/// - `param_types`: 参数类型数组
/// - `is_var_arg`: 是否为可变参数函数
pub fn function_type(
    return_type: LLVMTypeRef,
    param_types: &mut [LLVMTypeRef],
    is_var_arg: bool,
) -> LLVMTypeRef {
    unsafe {
        LLVMFunctionType(
            return_type,
            param_types.as_mut_ptr(),
            param_types.len() as u32,
            is_var_arg as i32,
        )
    }
}

/// 创建一个 LLVM 32 位整数常量
pub fn const_i32(context: &LlvmContext, value: u64) -> LLVMValueRef {
    unsafe { LLVMConstInt(context.i32_type(), value, 0) }
}

/// 检查基本块是否已有终结指令（ret、br 等）
///
/// 一个基本块只能有一个终结指令。在添加隐式 return 前应检查此条件。
pub fn block_has_terminator(block: LLVMBasicBlockRef) -> bool {
    unsafe { !LLVMGetBasicBlockTerminator(block).is_null() }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试：创建和销毁 LLVM 上下文
    #[test]
    fn test_context_lifecycle() {
        let ctx = LlvmContext::new();
        // 验证上下文指针非空
        assert!(!ctx.as_raw().is_null());
        // Drop 时自动释放
    }

    /// 测试：创建模块、函数，构建 IR，验证输出
    ///
    /// 综合验证 LlvmContext、LlvmModule、LlvmBuilder 和辅助函数。
    #[test]
    fn test_module_build_and_verify() {
        // 验证上下文创建
        let ctx = LlvmContext::new();
        assert!(!ctx.as_raw().is_null());

        // 验证模块和函数管理
        let module = LlvmModule::new("test", &ctx);
        let ret_type = ctx.i32_type();
        let fn_type = function_type(ret_type, &mut [], false);
        let func = module.add_function("main", fn_type);
        assert!(!func.is_null());
        assert!(module.get_function("main").is_some());
        assert!(module.get_function("nonexistent").is_none());

        // 验证 IR 构建
        let builder = LlvmBuilder::new(&ctx);
        let entry = append_basic_block(&ctx, func, "entry");
        builder.position_at_end(entry);
        builder.build_ret(const_i32(&ctx, 0));

        // 验证 IR 输出
        let ir = module.print_to_string();
        assert!(ir.contains("define i32 @main()"), "IR should contain main function");
        assert!(ir.contains("ret i32 0"), "IR should return 0");
    }

    /// 测试：set_target_metadata 正确设置 target triple 和 data layout
    ///
    /// 验证调用 set_target_metadata 后，IR 输出中包含 target triple 和 datalayout。
    #[test]
    fn test_set_target_metadata_adds_triple() {
        let ctx = LlvmContext::new();
        let module = LlvmModule::new("test_meta", &ctx);

        // 设置目标元数据
        module.set_target_metadata().expect("set_target_metadata should succeed");

        // 验证 IR 中包含 target triple
        let ir = module.print_to_string();
        assert!(
            ir.contains("target triple"),
            "IR should contain target triple after set_target_metadata"
        );
        assert!(
            ir.contains("target datalayout"),
            "IR should contain target datalayout after set_target_metadata"
        );
    }

    /// 测试：emit_ir_to_file 将 IR 写入指定文件
    ///
    /// 验证写入的 .ll 文件包含正确的 IR 内容。
    #[test]
    fn test_emit_ir_to_file() {
        let ctx = LlvmContext::new();
        let module = LlvmModule::new("test_emit", &ctx);

        // 创建一个简单的 main 函数
        let ret_type = ctx.i32_type();
        let fn_type = function_type(ret_type, &mut [], false);
        let func = module.add_function("main", fn_type);
        let builder = LlvmBuilder::new(&ctx);
        let entry = append_basic_block(&ctx, func, "entry");
        builder.position_at_end(entry);
        builder.build_ret(const_i32(&ctx, 42));

        // 写入临时 .ll 文件
        let temp_path = std::env::temp_dir().join(format!(
            "taoc_test_emit_ir_{}.ll",
            std::process::id(),
        ));
        module.emit_ir_to_file(&temp_path).expect("emit_ir_to_file should succeed");

        // 读取并验证文件内容
        let content = std::fs::read_to_string(&temp_path)
            .expect("should be able to read emitted IR file");
        assert!(content.contains("define i32 @main()"), "IR file should contain main function");
        assert!(content.contains("ret i32 42"), "IR file should contain ret i32 42");

        // 清理临时文件
        let _ = std::fs::remove_file(&temp_path);
    }
}
