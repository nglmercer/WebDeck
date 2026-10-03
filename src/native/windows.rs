use super::*;
use ::windows::core::{HSTRING, PCWSTR};
use ::windows::Win32::{
    Foundation::*,
    Media::Audio::{Endpoints::*, *},
    System::Com::*,
    UI::WindowsAndMessaging::*,
};
struct Com;
impl Com {
    fn init() -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(|_| Error::execution())?;
        }
        Ok(Self)
    }
}
impl Drop for Com {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
fn endpoint() -> Result<IAudioEndpointVolume> {
    unsafe {
        let e: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
            .map_err(|_| Error::execution())?;
        e.GetDefaultAudioEndpoint(eRender, eConsole)
            .and_then(|d| d.Activate(CLSCTX_ALL, None))
            .map_err(|_| Error::execution())
    }
}
pub(super) fn windows_volume(change: &VolumeChange) -> Result<Value> {
    let _com = Com::init()?;
    unsafe {
        let e = endpoint()?;
        let current = e
            .GetMasterVolumeLevelScalar()
            .map_err(|_| Error::execution())?;
        let target = percent(change, (current * 100.0).round() as i64) as f32 / 100.0;
        e.SetMasterVolumeLevelScalar(target, std::ptr::null())
            .map_err(|_| Error::execution())?;
    }
    Ok(json!({}))
}
pub(super) fn windows_endpoint(name: &str, mic: bool) -> Result<Value> {
    let flow = if mic { eCapture } else { eRender };
    if super::policy_config::set_default_by_friendly_name(flow, name, &[eConsole, eCommunications])
        .map_err(|_| Error::execution())?
    {
        Ok(json!({}))
    } else {
        Err(Error::execution())
    }
}
pub(super) fn windows_foreground(target: &str) -> Result<Value> {
    let s = HSTRING::from(target);
    unsafe {
        let hwnd = FindWindowW(None, PCWSTR(s.as_ptr())).map_err(|_| Error::execution())?;
        if !SetForegroundWindow(hwnd).as_bool() {
            return Err(Error::execution());
        }
    }
    Ok(json!({}))
}
pub(super) fn windows_display_off() -> Result<Value> {
    unsafe {
        SendMessageW(
            HWND_BROADCAST,
            WM_SYSCOMMAND,
            Some(WPARAM(SC_MONITORPOWER as usize)),
            Some(LPARAM(2)),
        );
    }
    Ok(json!({}))
}
pub(super) fn windows_app_volume(
    app: &str,
    change: &VolumeChange,
    context: &Context,
) -> Result<Value> {
    use ::windows::core::Interface;
    let _com = Com::init()?;
    unsafe {
        let e: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
            .map_err(|_| Error::execution())?;
        let d = e
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(|_| Error::execution())?;
        let manager: IAudioSessionManager2 = d
            .Activate(CLSCTX_ALL, None)
            .map_err(|_| Error::execution())?;
        let sessions = manager
            .GetSessionEnumerator()
            .map_err(|_| Error::execution())?;
        let n = sessions.GetCount().map_err(|_| Error::execution())?;
        let system = sysinfo::System::new_all();
        let mut found = false;
        for i in 0..n {
            context.check(Capability::Audio)?;
            let session = sessions.GetSession(i).map_err(|_| Error::execution())?;
            let control: IAudioSessionControl2 = session.cast().map_err(|_| Error::execution())?;
            let pid = control.GetProcessId().map_err(|_| Error::execution())?;
            if system
                .process(sysinfo::Pid::from_u32(pid))
                .is_some_and(|p| p.name().to_string_lossy().eq_ignore_ascii_case(app))
            {
                let volume: ISimpleAudioVolume = session.cast().map_err(|_| Error::execution())?;
                let current = volume.GetMasterVolume().map_err(|_| Error::execution())?;
                context.check(Capability::Audio)?;
                volume
                    .SetMasterVolume(
                        percent(change, (current * 100.0).round() as i64) as f32 / 100.0,
                        std::ptr::null(),
                    )
                    .map_err(|_| Error::execution())?;
                found = true;
            }
        }
        if found {
            Ok(json!({}))
        } else {
            Err(Error::execution())
        }
    }
}
