//! pycaw-equivalent glue (no Python counterpart).
//!
//! Default-endpoint switching via the undocumented `IPolicyConfig` COM API —
//! the same API `pycaw.AudioUtilities.SetDefaultDevice` uses. CLSID, IID,
//! vtable index (13), and signature verified against the real `pycaw`
//! sources (PyPI wheel, `pycaw/api/policyconfig/__init__.py` +
//! `pycaw/constants.py` + `pycaw/utils.py`):
//! - `CLSID_CPolicyConfigClient = {870af99c-171d-4f9e-af0d-e63df40c2bc9}`
//! - `IID_IPolicyConfig = {f8679f50-850a-41cf-9c72-430f290290c8}`
//! - `SetDefaultEndpoint(LPCWSTR device_id, DWORD role)` at vtable index 13
//! - default roles = `[eConsole]` (pycaw's `SetDefaultDevice` default)

#[cfg(windows)]
mod imp {
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::{
        EDataFlow, ERole, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize,
        StructuredStorage::PropVariantToString, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
    };
    use windows::core::{Interface as _, GUID, HRESULT, PCWSTR};

    const CLSID_POLICY_CONFIG_CLIENT: GUID = GUID::from_values(
        0x870af99c,
        0x171d,
        0x4f9e,
        [0xaf, 0x0d, 0xe6, 0x3d, 0xf4, 0x0c, 0x2b, 0xc9],
    );
    const IID_IPOLICY_CONFIG: GUID = GUID::from_values(
        0xf8679f50,
        0x850a,
        0x41cf,
        [0x9c, 0x72, 0x43, 0x0f, 0x29, 0x02, 0x90, 0xc8],
    );

    #[repr(C)]
    struct PolicyConfigVTable {
        _reserved: [*const std::ffi::c_void; 13],
        set_default_endpoint:
            unsafe extern "system" fn(*mut std::ffi::c_void, PCWSTR, u32) -> HRESULT,
    }

    unsafe fn set_default_endpoint_id(device_id: &str, role: ERole) -> windows::core::Result<()> {
        let unknown: windows::core::IUnknown =
            CoCreateInstance(&CLSID_POLICY_CONFIG_CLIENT, None, CLSCTX_ALL)?;
        let mut policy: *mut std::ffi::c_void = std::ptr::null_mut();
        unknown.query(&IID_IPOLICY_CONFIG, &mut policy).ok()?;
        // Take ownership so the interface is Released on drop.
        let _guard = windows::core::IUnknown::from_raw(policy);
        let vtable = *(policy as *const *const PolicyConfigVTable);
        let wide: Vec<u16> = device_id.encode_utf16().chain(std::iter::once(0)).collect();
        ((*vtable).set_default_endpoint)(policy, PCWSTR(wide.as_ptr()), role.0 as u32).ok()
    }

    /// Enumerate active endpoints on `flow`, set the one whose friendly name
    /// exactly equals `name` as default for `roles`. Returns whether a device
    /// matched (mirrors pycaw's silent no-match).
    pub fn set_default_by_friendly_name(
        flow: EDataFlow,
        name: &str,
        roles: &[ERole],
    ) -> Result<bool, String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let result = (|| -> windows::core::Result<bool> {
            unsafe {
                let enumerator: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
                let collection = enumerator.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE)?;
                let count = collection.GetCount()?;
                for i in 0..count {
                    let device = collection.Item(i)?;
                    let store = device.OpenPropertyStore(STGM_READ)?;
                    let variant = store.GetValue(&PKEY_Device_FriendlyName)?;
                    let mut buffer = [0u16; 256];
                    PropVariantToString(&variant, &mut buffer)?;
                    let length = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
                    let friendly = String::from_utf16_lossy(&buffer[..length]);
                    if friendly == name {
                        let id = device.GetId()?;
                        let mut length = 0;
                        while *id.0.add(length) != 0 {
                            length += 1;
                        }
                        let id = String::from_utf16_lossy(std::slice::from_raw_parts(
                            id.0, length,
                        ));
                        for role in roles {
                            set_default_endpoint_id(&id, *role)?;
                        }
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        })();
        unsafe {
            CoUninitialize();
        }
        result.map_err(|e| e.to_string())
    }
}

#[cfg(windows)]
pub use imp::set_default_by_friendly_name;
