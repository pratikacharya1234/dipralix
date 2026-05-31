import { writable } from 'svelte/store';

export const dipralixEvents = writable([]);

export function emitDipralixEvent(type, data = {}) {
  dipralixEvents.update(events => [...events, { type, data, timestamp: Date.now() }]);
}
