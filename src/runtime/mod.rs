mod adapter;
mod bridge;
pub mod capabilities;
mod errors;
mod modules;
mod vm;

pub use adapter::VmAdapter;
pub use vm::VmRuntime;
pub const NAPI_VM_REVISION: &str = "881cc8f1cec262049b6d31fa499b0d601ba673c5";
