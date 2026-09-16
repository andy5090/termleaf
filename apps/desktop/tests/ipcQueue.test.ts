import assert from 'node:assert/strict';
import test from 'node:test';
import { IpcQueue } from '../src/ipcQueue.ts';

test('stateful commands finish in issue order', async () => {
  const queue = new IpcQueue();
  const events: string[] = [];
  let releaseFirst: (() => void) | undefined;

  const first = queue.run(async () => {
    events.push('first:start');
    await new Promise<void>(resolve => { releaseFirst = resolve; });
    events.push('first:end');
  });
  const second = queue.run(async () => { events.push('second'); });

  await Promise.resolve();
  assert.deepEqual(events, ['first:start']);
  releaseFirst?.();
  await Promise.all([first, second]);
  assert.deepEqual(events, ['first:start', 'first:end', 'second']);
});

test('a failed command does not poison later commands', async () => {
  const queue = new IpcQueue();
  const failed = queue.run(async () => { throw new Error('expected'); });
  const recovered = queue.run(async () => 'ready');

  await assert.rejects(failed, /expected/);
  assert.equal(await recovered, 'ready');
});
