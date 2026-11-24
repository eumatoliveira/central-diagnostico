import assert from 'node:assert/strict';
import test from 'node:test';
import { summarizeDiagnostic } from '../src/diagnostic.mjs';

test('diagnostic import retains only the supported aggregate fields', () => {
  const data = {
    time: '2020-03-01T10:00:00.000Z',
    ram: { percent: 42, detail: 'private' },
    disk: { free: 30, total: 100 },
    events: [{ status: 'Requer revisão', detail: 'private' }, { status: 'Ativo' }, { status: 'Indisponível' }],
  };
  assert.deepEqual(summarizeDiagnostic(data), {
    collected_at: data.time, memory_used: 42, disk_used: 70,
    checks_total: 3, checks_review: 1, checks_unavailable: 1,
  });
  assert.equal(summarizeDiagnostic({ ...data, disk: null }).disk_used, null);
  assert.throws(() => summarizeDiagnostic({ ...data, time: '2020-02-31T10:00:00.000Z' }));
  assert.throws(() => summarizeDiagnostic({ ...data, ram: { percent: 1000 } }));
  assert.throws(() => summarizeDiagnostic({ ...data, disk: { free: 50, total: 20 } }));
});
