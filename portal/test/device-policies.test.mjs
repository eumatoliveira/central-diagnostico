import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { PGlite } from '@electric-sql/pglite';

test('device inventory enforces ownership and limited write permissions', async () => {
  const database = new PGlite();
  try {
    await database.exec(`
      create role anon;
      create role authenticated;
      create schema auth;
      create table auth.users (id uuid primary key);
      create function auth.uid() returns uuid language sql stable as
        $$ select nullif(current_setting('request.jwt.claim.sub', true), '')::uuid $$;
      grant usage on schema auth to authenticated;
      insert into auth.users values
        ('10000000-0000-0000-0000-000000000001'),
        ('10000000-0000-0000-0000-000000000002');
    `);
    for (const migration of ['202611230001_devices.sql', '202611230002_diagnostics.sql']) {
      await database.exec(await readFile(new URL(`../supabase/migrations/${migration}`, import.meta.url), 'utf8'));
    }
    await database.exec(`set role anon`);
    await assert.rejects(database.query('select * from public.central_devices'), { code: '42501' });
    await assert.rejects(database.query('select * from public.central_diagnostics'), { code: '42501' });
    await database.exec(`set role authenticated; set request.jwt.claim.sub = '10000000-0000-0000-0000-000000000001'`);
    await database.query("insert into public.central_devices (label, platform) values ('Primeiro computador', 'windows')");
    const { rows: [{ id: firstDevice }] } = await database.query('select id from public.central_devices');
    const insertDiagnostic = 'insert into public.central_diagnostics (device_id, collected_at, memory_used, checks_total, checks_review, checks_unavailable) values ($1, now(), 25, 3, 1, 1)';
    await database.query(insertDiagnostic, [firstDevice]);
    assert.equal((await database.query('select memory_used from public.central_diagnostics')).rows[0].memory_used, 25);
    await assert.rejects(database.query("insert into public.central_devices (owner_id, label, platform) values ('10000000-0000-0000-0000-000000000002', 'Adulterado', 'linux')"), { code: '42501' });
    await database.exec(`set request.jwt.claim.sub = '10000000-0000-0000-0000-000000000002'`);
    assert.deepEqual((await database.query('select label from public.central_devices')).rows, []);
    assert.deepEqual((await database.query('select memory_used from public.central_diagnostics')).rows, []);
    await assert.rejects(database.query(insertDiagnostic, [firstDevice]), { code: '23503' });
    await database.query("insert into public.central_devices (label, platform) values ('Segundo computador', 'linux')");
    assert.deepEqual((await database.query('select label from public.central_devices')).rows, [{ label: 'Segundo computador' }]);
    await assert.rejects(database.query("update public.central_devices set label = 'Alterado'"), { code: '42501' });
  } finally {
    await database.close();
  }
});
