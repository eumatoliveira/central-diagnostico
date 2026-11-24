begin;

alter table public.central_devices add constraint central_devices_owned_id unique (owner_id, id);

create table public.central_diagnostics (
  device_id uuid not null,
  owner_id uuid not null default auth.uid(),
  collected_at timestamptz not null,
  memory_used smallint not null check (memory_used between 0 and 100),
  disk_used smallint check (disk_used between 0 and 100),
  checks_total smallint not null check (checks_total between 0 and 1000),
  checks_review smallint not null check (checks_review between 0 and checks_total),
  checks_unavailable smallint not null check (checks_unavailable between 0 and checks_total - checks_review),
  primary key (device_id, collected_at),
  foreign key (owner_id, device_id) references public.central_devices (owner_id, id) on delete cascade
);

alter table public.central_diagnostics enable row level security;
revoke all on table public.central_diagnostics from public, anon, authenticated;
grant select on table public.central_diagnostics to authenticated;
grant insert (device_id, collected_at, memory_used, disk_used, checks_total, checks_review, checks_unavailable)
  on table public.central_diagnostics to authenticated;

create policy central_diagnostics_read_own on public.central_diagnostics
  for select to authenticated using ((select auth.uid()) = owner_id);

create policy central_diagnostics_insert_own on public.central_diagnostics
  for insert to authenticated with check ((select auth.uid()) = owner_id);

commit;
