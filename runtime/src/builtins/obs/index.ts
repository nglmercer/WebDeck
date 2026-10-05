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
  get_scenes: "GetSceneList",
  get_current_scene: "GetCurrentProgramScene",
  get_inputs: "GetInputList",
  get_hotkeys: "GetHotkeyList",
  get_stream_status: "GetStreamStatus",
  get_recording_status: "GetRecordStatus",
  get_virtual_camera_status: "GetVirtualCamStatus",
  input_mute: "SetInputMute",
  input_unmute: "SetInputMute",
};
// Query actions report OBS state; control actions stay response-free for parity with v1 behavior.
const queries = new Set([
  "get_version",
  "get_scenes",
  "get_current_scene",
  "get_inputs",
  "get_hotkeys",
  "get_stream_status",
  "get_recording_status",
  "get_virtual_camera_status",
]);
function requestData(action: string, target: string): unknown {
  if (action === "scene") return { sceneName: target };
  if (action === "hotkey")
    return {
      keyId: target,
      keyModifiers: { shift: false, control: false, alt: false, command: false },
    };
  if (action === "input_mute" || action === "input_unmute")
    return { inputName: target, inputMuted: action === "input_mute" };
  return {};
}
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
    const send = (requestType: string, data: unknown): any => {
      call("network.wsSend", {
        id,
        value: { op: 6, d: { requestType, requestId: "webdeck", requestData: data } },
      });
      for (let turns = 0; turns < 64; turns++) {
        const response = call("network.wsReceive", { id });
        if (response.op === 7 && response.d.requestId === "webdeck") {
          if (!response.d.requestStatus.result)
            throw new Error("OBS action failed");
          return response.d.responseData ?? {};
        }
      }
      throw new Error("OBS response budget exhausted");
    };
    const action = command.action;
    if (action === "input_toggle_mute") {
      const before = send("GetInputMute", { inputName: command.target });
      const muted = !before.inputMuted;
      send("SetInputMute", {
        inputName: command.target,
        inputMuted: muted,
      });
      return { inputName: command.target, inputMuted: muted };
    }
    const requestType = requests[action];
    if (!requestType) throw new Error("Invalid OBS action");
    const data = send(requestType, requestData(action, command.target));
    if (action === "get_inputs") {
      // Mute state lives on a separate request; bound the fan-out for large scenes.
      const inputs = (Array.isArray(data.inputs) ? data.inputs : [])
        .slice(0, 32)
        .map((input: any) => {
          const mute = send("GetInputMute", { inputName: input.inputName });
          return {
            inputName: input.inputName,
            inputKind: input.inputKind,
            inputMuted: !!mute.inputMuted,
          };
        });
      return { inputs };
    }
    return queries.has(action) ? data : {};
  } finally {
    call("network.wsClose", { id });
  }
}
