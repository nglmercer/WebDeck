import { call } from "webdeck:host";
const requests = {
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
    hotkey: "TriggerHotkeyByName",
    hotkey_sequence: "TriggerHotkeyByKeySequence",
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
function requestData(command) {
    const { action, target } = command;
    if (action === "scene")
        return command.scene_uuid ? { sceneUuid: command.scene_uuid } : { sceneName: target };
    if (action === "hotkey")
        return { hotkeyName: target, ...(command.context_name ? { contextName: command.context_name } : {}) };
    if (action === "hotkey_sequence")
        return {
            keyId: target,
            keyModifiers: command.key_modifiers ?? { shift: false, control: false, alt: false, command: false },
        };
    if (action === "input_mute" || action === "input_unmute")
        return { inputName: target, inputMuted: action === "input_mute" };
    return {};
}
export function execute(command) {
    const send = (requestType, requestData) => call("obs.request", { requestType, requestData });
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
    if (!requestType)
        throw new Error("Invalid OBS action");
    const data = send(requestType, requestData(command));
    if (action === "get_inputs") {
        // Mute state lives on a separate request; bound the fan-out for large scenes.
        const inputs = (Array.isArray(data.inputs) ? data.inputs : [])
            .slice(0, 32)
            .map((input) => {
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
}
