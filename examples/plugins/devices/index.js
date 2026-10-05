// Demonstration package: metadata powers discovery, health and button generation.
import {set} from 'webdeck:storage';
const devices = [{id: 'a', name: 'Alpha'}, {id: 'b', name: 'Beta'}];
export function invoke_action(action, args) {
  if (action === 'discover') return {devices};
  if (action === 'health') return {ready: true};
  if (action === 'select') {
    if (!devices.some(device => device.id === args.device)) throw new Error('Unknown device');
    set('selected', args.device);
    return {};
  }
  throw new Error('Unknown action');
}
