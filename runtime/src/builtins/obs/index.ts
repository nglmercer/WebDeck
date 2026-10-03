import { call } from "webdeck:host";
import type { Command } from "../../generated/contracts";
type ObsCommand = Extract<Command, { type: "obs" }>;
const requests: Record<string, string> = {
  get_version: "GetVersion",
  toggle_recording: "ToggleRecord",
  start_recording: "StartRecord",
  stop_recording: "StopRecord",
  toggle_recording_pause: "ToggleRecordPause",
  pause_recording: "PauseRecord",
  resume_recording: "ResumeRecord",
  toggle_stream: "ToggleStream",
  start_stream: "StartStream",
  stop_stream: "StopStream",
  toggle_virtual_camera: "ToggleVirtualCam",
  start_virtual_camera: "StartVirtualCam",
  stop_virtual_camera: "StopVirtualCam",
  scene: "SetCurrentProgramScene",
  hotkey: "TriggerHotkeyByKeySequence",
};
export function execute(command: ObsCommand): unknown {
  const settings = call("secrets.integration", { id: "obs" });
  const host = settings.host.includes(":")
    ? `[${settings.host}]`
    : settings.host;
  const id = call("network.wsOpen", { url: `ws://${host}:${settings.port}` });
  try {
    const hello = call("network.wsReceive", { id });
    if (hello.op !== 0) throw new Error("OBS handshake failed");
    const identify: any = { rpcVersion: 1, eventSubscriptions: 0 };
    if (hello.d.authentication) {
      const secret = call("crypto.sha256Base64", {
        text: settings.password + hello.d.authentication.salt,
      });
      identify.authentication = call("crypto.sha256Base64", {
        text: secret + hello.d.authentication.challenge,
      });
    }
    call("network.wsSend", { id, value: { op: 1, d: identify } });
    if (call("network.wsReceive", { id }).op !== 2)
      throw new Error("OBS identification failed");
    const requestType = requests[command.action];
    if (!requestType) throw new Error("Invalid OBS action");
    const requestData =
      command.action === "scene"
        ? { sceneName: command.target }
        : command.action === "hotkey"
          ? {
              keyId: command.target,
              keyModifiers: {
                shift: false,
                control: false,
                alt: false,
                command: false,
              },
            }
          : {};
    call("network.wsSend", {
      id,
      value: { op: 6, d: { requestType, requestId: "webdeck", requestData } },
    });
    for (let turns = 0; turns < 64; turns++) {
      const response = call("network.wsReceive", { id });
      if (response.op === 7 && response.d.requestId === "webdeck") {
        if (!response.d.requestStatus.result)
          throw new Error("OBS action failed");
        return command.action === "get_version" ? response.d.responseData : {};
      }
    }
    throw new Error("OBS response budget exhausted");
  } finally {
    call("network.wsClose", { id });
  }
}
