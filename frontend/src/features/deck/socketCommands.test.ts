import { afterEach, describe, expect, it, vi } from 'vitest';
import { commandRequestId, SocketCommandTracker } from './socketCommands';

afterEach(() => vi.useRealTimers());
describe('socket command observation', () => {
  it('correlates completion and failure without treating acceptance as completion', () => {
    vi.useFakeTimers(); const report = vi.fn(); const tracker = new SocketCommandTracker(report, 100);
    tracker.track({message: '/key A', request_id: 'one'});
    tracker.result({api_version:2, request_id:'unrelated', state:'failed', code:'forbidden', message:'denied'});
    tracker.result({api_version:2, request_id:'one', state:'accepted'});
    expect(report).not.toHaveBeenCalled();
    tracker.result({api_version:2, request_id:'one', state:'completed', result:{}});
    vi.advanceTimersByTime(100); expect(report).not.toHaveBeenCalled();
    tracker.track({message:'/key B',request_id:'two'});
    tracker.result({api_version:2,request_id:'two',state:'failed',code:'forbidden',message:'denied'});
    expect(report).toHaveBeenCalledExactlyOnceWith('denied');
  });
  it('reports uncertainty once on timeout/disconnect and clears its timers', () => {
    vi.useFakeTimers(); const report = vi.fn(); const tracker = new SocketCommandTracker(report, 100);
    tracker.track({message:'/key A',request_id:'one'});
    vi.advanceTimersByTime(100);
    expect(report).toHaveBeenCalledTimes(1); expect(report.mock.calls[0]?.[0]).toContain('unknown');
    tracker.track({message:'/key B',request_id:'two'}); tracker.disconnect(); tracker.disconnect();
    vi.advanceTimersByTime(1000); expect(report).toHaveBeenCalledTimes(2);
    tracker.result({api_version:2,request_id:'two',state:'completed',result:{}});
    expect(report).toHaveBeenCalledTimes(2);
  });
  it('generates schema-safe correlation IDs without the secure-context randomUUID API', () => {
    expect(commandRequestId()).toMatch(/^[a-f0-9]{32}$/);
    expect(commandRequestId()).not.toBe(commandRequestId());
  });
});
