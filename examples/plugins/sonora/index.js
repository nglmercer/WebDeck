// Controls a running Sonora through its control socket. Each action sends one request through
// the operation for its kind: sonora.query (read), sonora.control (audio) or sonora.remote
// (network). The host checks the action's grant and the request's shape again before sending.
import {call} from 'webdeck:host';

const SWITCH = ['on', 'off', 'toggle'];
const KINDS = ['song', 'album', 'artist', 'playlist'];

function ask(operation, request) {
  return call(operation, {request});
}
function oneOf(value, allowed, name) {
  if (allowed.indexOf(value) < 0) throw new Error(name + ' must be ' + allowed.join(', '));
  return value;
}
function percent(value) {
  if (typeof value !== 'number' || value < 0 || value > 100) throw new Error('Percent must be between 0 and 100');
  return Math.round(value);
}
function text(value) {
  if (typeof value !== 'string' || value.length === 0 || value.length > 200) throw new Error('Text must have 1 to 200 characters');
  return value;
}
function link(value) {
  if (typeof value !== 'string' || value.length === 0 || value.length > 2048) throw new Error('Link must have 1 to 2048 characters');
  return value;
}

export function invoke_action(action, args) {
  if (action === 'current') return ask('sonora.query', {playback: 'current-track'});
  if (action === 'queue') return ask('sonora.query', {queue: 'snapshot'});
  if (action === 'lyrics') return ask('sonora.query', 'lyrics');
  if (action === 'providers') return ask('sonora.query', {provider: 'list'});

  if (action === 'play' || action === 'pause' || action === 'toggle' || action === 'next' || action === 'previous') {
    return ask('sonora.control', {playback: action});
  }
  if (action === 'seek') return ask('sonora.control', {playback: {seek: percent(args.percent)}});
  if (action === 'volume') return ask('sonora.control', {playback: {volume: {to: percent(args.percent)}}});
  if (action === 'mute') return ask('sonora.control', {playback: {mute: oneOf(args.state, SWITCH, 'State')}});
  if (action === 'shuffle') return ask('sonora.control', {playback: {shuffle: oneOf(args.state, SWITCH, 'State')}});
  if (action === 'repeat') return ask('sonora.control', {playback: {repeat: oneOf(args.mode, ['off', 'all', 'one'], 'Mode')}});
  if (action === 'like') return ask('sonora.control', {like: oneOf(args.state, SWITCH, 'State')});
  if (action === 'provider') return ask('sonora.control', {provider: {current: text(args.slug)}});

  if (action === 'search') {
    return ask('sonora.remote', {search: {text: text(args.text), kind: oneOf(args.kind, KINDS, 'Kind'), limit: 5}});
  }
  if (action === 'search_play') {
    return ask('sonora.remote', {queue: {play: [{search: {text: text(args.text), kind: oneOf(args.kind, KINDS, 'Kind')}}]}});
  }
  if (action === 'queue_link') return ask('sonora.remote', {queue: {add: [{link: link(args.link)}]}});
  if (action === 'play_link') return ask('sonora.remote', {queue: {play: [{link: link(args.link)}]}});

  throw new Error('Unknown action');
}
