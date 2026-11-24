begin;

create table public.central_devices (
  id uuid primary key default gen_random_uuid(),
  owner_id uuid not null default auth.uid() references auth.users(id) on delete cascade,
  label text not null check (label = btrim(label) and char_length(label) between 1 and 80),
  platform text not null check (platform in ('windows', 'linux', 'wsl2')),
  created_at timestamptz not null default now()
);

create index central_devices_owner_created_idx on public.central_devices (owner_id, created_at desc);
alter table public.central_devices enable row level security;
revoke all on table public.central_devices from public, anon, authenticated;
grant select on table public.central_devices to authenticated;
grant insert (label, platform) on table public.central_devices to authenticated;

create policy central_devices_read_own on public.central_devices
  for select to authenticated using ((select auth.uid()) = owner_id);

create policy central_devices_insert_own on public.central_devices
  for insert to authenticated with check ((select auth.uid()) = owner_id);

commit;
