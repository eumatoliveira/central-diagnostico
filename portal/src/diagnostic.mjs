export const maxDiagnosticBytes = 256 * 1024;

const statuses = new Set(['Ativo', 'Concluído', 'Consultado', 'Requer revisão', 'Indisponível']);
const percentage = (value) => Number.isInteger(value) && value >= 0 && value <= 100;

export function summarizeDiagnostic(report) {
  const clock = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/;
  if (!report || typeof report.time !== 'string' || !clock.test(report.time)
      || !percentage(report.ram?.percent) || !Array.isArray(report.events) || report.events.length > 1000) {
    throw new Error('Formato de diagnóstico inválido');
  }
  const date = new Date(report.time);
  if (!Number.isFinite(+date) || date.toISOString() !== report.time || +date > Date.now() + 300000) {
    throw new Error('Horário de coleta inválido');
  }
  const disk = report.disk;
  if (disk != null && (!Number.isFinite(disk.free) || !Number.isFinite(disk.total)
      || disk.free < 0 || disk.total < disk.free)) {
    throw new Error('Capacidade de armazenamento inválida');
  }
  let checksReview = 0;
  let checksUnavailable = 0;
  for (const event of report.events) {
    if (!statuses.has(event?.status)) throw new Error('Estado de diagnóstico inválido');
    if (event.status === 'Requer revisão') checksReview++;
    if (event.status === 'Indisponível') checksUnavailable++;
  }
  return {
    collected_at: report.time,
    memory_used: report.ram.percent,
    disk_used: disk?.total ? Math.round(100 * (1 - disk.free / disk.total)) : null,
    checks_total: report.events.length,
    checks_review: checksReview,
    checks_unavailable: checksUnavailable,
  };
}
