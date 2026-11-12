mod adapter;
mod bridge;
pub mod capabilities;
mod errors;
mod modules;
pub mod plugins;
mod sandbox;
mod vm;

pub use adapter::VmAdapter;
pub use vm::VmRuntime;
pub const NAPI_VM_REVISION: &str = "29d7c77c7645c85d3c91911814831e36d919550f";
