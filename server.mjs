import { createServer } from 'node:http';
import { readFile, statfs } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { randomBytes } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { win32 } from 'node:path';
import os from 'node:os';
import { formats, renderReport } from './reports.mjs';
import { maxScanBytes, scanBuffer } from './clamav.mjs';
import { isS3Configured, runBackup } from './backup/restic.mjs';

const execute = promisify(execFile);
const port = 47831;
const host = `127.0.0.1:${port}`;
const token = randomBytes(32).toString('hex');
const windows = process.platform === 'win32';
const started = new Date().toISOString();
const gib = (bytes) => Math.round(bytes / 2 ** 30 * 10) / 10;
const deviceCapacity = (bytes) => !Number.isFinite(bytes) || bytes < 0 ? 'Capacidade indisponível' :
  bytes < 2 ** 30 ? `${Math.round(bytes / 2 ** 20)} MiB` : `${gib(bytes)} GiB`;
const agentPath = fileURLToPath(new URL(`./agente/target/debug/central-agente${windows ? '.exe' : ''}`, import.meta.url));
const systemPath = windows && process.env.SystemRoot ? win32.join(process.env.SystemRoot, 'System32') : null;
const powershellPath = systemPath && win32.join(systemPath, 'WindowsPowerShell', 'v1.0', 'powershell.exe');

async function networkInventory() {
  const probes = systemPath ? [
    ['Interfaces', win32.join(systemPath, 'ipconfig.exe'), ['/all']],
    ['Rotas', win32.join(systemPath, 'route.exe'), ['print']],
    ['Adaptadores', powershellPath, ['-NoProfile', '-NonInteractive', '-Command', 'Get-NetAdapter | Select-Object Name, Status, LinkSpeed | Out-String -Width 180']],
  ] : process.platform === 'linux' ? [
    ['Interfaces', '/usr/sbin/ip', ['-brief', 'address', 'show']],
    ['Rotas IPv4', '/usr/sbin/ip', ['-4', 'route', 'show']],
    ['Rotas IPv6', '/usr/sbin/ip', ['-6', 'route', 'show']],
  ] : [];
  return Promise.all(probes.map(async ([title, program, args]) => {
    try {
      const { stdout } = await execute(program, args, { timeout: 5000, maxBuffer: 65536, windowsHide: true, encoding: 'buffer' });
      const codec = title === 'Adaptadores' ? 'utf-8' : windows ? 'ibm850' : 'utf-8';
      let output;
      try { output = new TextDecoder(codec).decode(stdout); } catch { output = stdout.toString(); }
      return { title, output: output.trim() || 'Nenhuma entrada.' };
    } catch {
      return { title, output: 'Consulta indisponível neste sistema ou sem resposta.' };
    }
  }));
}

async function gpuInventory() {
  const program = systemPath ? win32.join(systemPath, 'nvidia-smi.exe') : process.platform === 'linux' ? '/usr/bin/nvidia-smi' : null;
  if (!program) return { area: 'GPU NVIDIA', status: 'Indisponível', detail: 'Consulta sem suporte neste sistema' };
  try {
    const { stdout } = await execute(program, ['--query-gpu=name,driver_version,temperature.gpu,memory.used,memory.total,fan.speed,clocks.current.graphics', '--format=csv,noheader'], { timeout: 5000, maxBuffer: 16384, windowsHide: true });
    if (stdout.trim()) return { area: 'GPU NVIDIA', status: 'Consultado', detail: `Modelo, driver, temperatura, memória usada e total, comando da ventoinha e clock gráfico: ${stdout.trim().split(/\r?\n/).join(' | ')}` };
  } catch {
    // Driver ou utilitário podem estar ausentes ou indisponíveis.
  }
  return { area: 'GPU NVIDIA', status: 'Indisponível', detail: 'Nenhuma telemetria retornada pelo nvidia-smi no local padrão' };
}

async function windowsDeviceIssues() {
  if (!powershellPath) return [];
  const command = '$ErrorActionPreference="Stop"; [Console]::OutputEncoding=[Text.Encoding]::UTF8; ConvertTo-Json -InputObject @(Get-CimInstance Win32_PnPEntity -Filter "Present = TRUE AND ConfigManagerErrorCode <> 0" | Select-Object -First 17 Name,ConfigManagerErrorCode) -Compress';
  const { stdout } = await execute(powershellPath, ['-NoProfile', '-NonInteractive', '-Command', command], { timeout: 8000, maxBuffer: 32768, windowsHide: true });
  const devices = JSON.parse(stdout);
  if (!devices.length) return [{ area: 'Dispositivos e drivers', status: 'Consultado', detail: 'O Windows não reportou problemas nos dispositivos presentes consultados' }];
  const reasons = new Map([[10, 'O dispositivo não iniciou'], [22, 'O dispositivo está desativado'], [28, 'Driver não instalado'], [31, 'O Windows não conseguiu carregar os drivers necessários'], [43, 'O Windows interrompeu o dispositivo após um problema reportado']]);
  const findings = devices.slice(0, 16).map(({ Name, ConfigManagerErrorCode: code }) => ({
    area: 'Dispositivos e drivers', status: 'Requer revisão',
    detail: `${Name || 'Dispositivo sem nome'} · Código ${code} · ${reasons.get(code) || 'Consulte o Gerenciador de Dispositivos para ver a causa informada'}. Verifique o Windows Update ou o suporte oficial do fabricante.`,
  }));
  if (devices.length > 16) findings.push({ area: 'Dispositivos e drivers', status: 'Requer revisão', detail: 'Há outros dispositivos com problemas. Consulte o Gerenciador de Dispositivos.' });
  return findings;
}

async function windowsApplicationIssues() {
  if (!powershellPath) return [];
  const command = [
    '$ErrorActionPreference="Stop"',
    '[Console]::OutputEncoding=[Text.Encoding]::UTF8',
    '$filter=@{LogName="Application"; ProviderName="Application Error","Application Hang","SideBySide"; Id=1000,1002,33,59; StartTime=(Get-Date).AddDays(-7)}',
    'try { $events=@(Get-WinEvent -FilterHashtable $filter -MaxEvents 101) } catch { if ($_.FullyQualifiedErrorId -notlike "NoMatchingEventsFound,*") { throw }; $events=@() }',
    '$result=@($events | ForEach-Object { $app=$null; if ($_.ProviderName -in "Application Error","Application Hang") { $xml=[xml]$_.ToXml(); $app=($xml.Event.EventData.Data | Where-Object Name -eq AppName | Select-Object -First 1)."#text" }; @{provider=$_.ProviderName; id=$_.Id; app=$app} })',
    'ConvertTo-Json -InputObject $result -Compress',
  ].join('; ');
  const { stdout } = await execute(powershellPath, ['-NoProfile', '-NonInteractive', '-Command', command], { timeout: 8000, maxBuffer: 65536, windowsHide: true });
  const entries = JSON.parse(stdout);
  const groups = new Map();
  for (const { provider, id, app } of entries.slice(0, 100)) {
    let issue;
    if (provider === 'Application Error' && id === 1000) issue = 'Encerramento inesperado';
    if (provider === 'Application Hang' && id === 1002) issue = 'Programa parou de responder';
    if (provider === 'SideBySide' && [33, 59].includes(id)) issue = 'Falha de configuração de dependências';
    if (!issue) continue;
    const executable = typeof app === 'string' && /^[\p{L}\p{N} _().+-]{1,100}\.exe$/iu.test(app) ? ` em ${app}` : '';
    const label = `${issue}${executable}`;
    groups.set(label, (groups.get(label) ?? 0) + 1);
  }
  if (!groups.size) return [{ area: 'Falhas de aplicativos', status: 'Consultado', detail: 'Nenhum evento dos tipos consultados encontrado nos últimos sete dias' }];
  const findings = [...groups].slice(0, 12).map(([label, count]) => ({
    area: 'Falhas de aplicativos', status: 'Requer revisão', detail: `${label} · ${count} registro(s) nos últimos sete dias. Use a opção de reparo ou atualização do fornecedor se o problema se repetir.`,
  }));
  if (entries.length > 100 || groups.size > 12) findings.push({ area: 'Falhas de aplicativos', status: 'Requer revisão', detail: 'A lista foi limitada. Consulte o Visualizador de Eventos para ver os demais registros.' });
  return findings;
}

async function diskInventory() {
  if (powershellPath) {
    const command = '$ErrorActionPreference="Stop"; [Console]::OutputEncoding=[Text.Encoding]::UTF8; ConvertTo-Json -InputObject @(Get-PhysicalDisk | Select-Object -First 32 FriendlyName,MediaType,HealthStatus,OperationalStatus,Size) -Compress';
    const { stdout } = await execute(powershellPath, ['-NoProfile', '-NonInteractive', '-Command', command], { timeout: 8000, maxBuffer: 32768, windowsHide: true });
    return JSON.parse(stdout).map((disk, index) => {
      const flags = [disk.OperationalStatus].flat().filter((value) => typeof value === 'string');
      const health = disk.HealthStatus;
      const review = ['Warning', 'Unhealthy'].includes(health) || flags.some((flag) => ['Degraded', 'Lost Communication', 'Predictive Failure', 'Error'].includes(flag));
      const status = review ? 'Requer revisão' : health === 'Healthy' ? 'Consultado' : 'Indisponível';
      const size = deviceCapacity(disk.Size);
      return { area: `Disco ${index + 1}`, status, detail: `${disk.FriendlyName || 'Modelo não informado'} · ${disk.MediaType || 'Tipo não informado'} · ${size} · Estado informado: ${health || 'indisponível'}${flags.length ? ` (${flags.join(', ')})` : ''}` };
    });
  }
  if (process.platform === 'linux') {
    const { stdout } = await execute('/usr/bin/lsblk', ['--json', '--nodeps', '--bytes', '--output', 'TYPE,MODEL,SIZE'], { timeout: 5000, maxBuffer: 32768 });
    return JSON.parse(stdout).blockdevices.filter(({ type }) => type === 'disk').slice(0, 32).map((disk, index) => ({
      area: `Disco ${index + 1}`,
      status: 'Indisponível',
      detail: `${disk.model?.trim() || 'Modelo não informado'} · ${deviceCapacity(disk.size)} · Integridade não consultada pelo inventário de dispositivos Linux`,
    }));
  }
  return [];
}

async function auditBaseline() {
  const { stdout } = await execute(agentPath, ['auditar'], { timeout: 10000, maxBuffer: 32768, windowsHide: true });
  const checks = JSON.parse(stdout);
  if (!Array.isArray(checks) || !checks.length || !checks.every((check) =>
    typeof check.title === 'string' && typeof check.evidence === 'string' && typeof check.action === 'string' &&
    ['Ativo', 'Requer revisão', 'Indisponível'].includes(check.status))) {
    throw new Error('Resultado de auditoria inválido');
  }
  return checks.map(({ title, status, evidence, action }) => ({
    area: title,
    status,
    detail: status === 'Ativo' ? evidence : `${evidence}. ${action}`,
  }));
}

async function defender() {
  if (!powershellPath) return [];
  const { stdout } = await execute(powershellPath, ['-NoProfile', '-NonInteractive', '-Command', '$ErrorActionPreference="Stop"; Get-MpComputerStatus | Select-Object AMRunningMode,AMServiceEnabled,AntivirusEnabled,RealTimeProtectionEnabled,DefenderSignaturesOutOfDate,AntivirusSignatureVersion,IsTamperProtected | ConvertTo-Json -Compress'], { timeout: 8000, maxBuffer: 32768, windowsHide: true });
  const state = JSON.parse(stdout);
  const finding = (area, flag, active, inactive) => ({
    area,
    status: flag === true ? 'Ativo' : flag === false ? 'Requer revisão' : 'Indisponível',
    detail: flag === true ? active : flag === false ? inactive : 'O Defender não retornou este indicador',
  });
  const signature = typeof state.AntivirusSignatureVersion === 'string' && /^\d+(\.\d+){1,5}$/.test(state.AntivirusSignatureVersion) ? ` Versão ${state.AntivirusSignatureVersion}.` : '';
  const current = typeof state.DefenderSignaturesOutOfDate === 'boolean' ? !state.DefenderSignaturesOutOfDate : undefined;
  const mode = state.AMRunningMode === 'Passive Mode' ? ' O Defender está em modo passivo.' : '';
  return [
    finding('Proteção em tempo real', state.RealTimeProtectionEnabled, 'Proteção em tempo real do Defender ativa', `Proteção em tempo real do Defender desativada.${mode}`),
    finding('Serviço Defender', state.AMServiceEnabled, 'Serviço do Microsoft Defender habilitado', 'Serviço do Microsoft Defender desabilitado'),
    finding('Antivírus Defender', state.AntivirusEnabled, 'Componente antivírus habilitado', 'Componente antivírus desabilitado'),
    finding('Assinaturas Defender', current, `O Defender indica assinaturas atualizadas.${signature}`, `O Defender indica assinaturas desatualizadas.${signature}`),
    finding('Proteção contra adulteração', state.IsTamperProtected, 'Proteção contra adulteração ativa', 'Proteção contra adulteração desativada'),
  ];
}

async function diagnose() {
  const [disk, protection, baseline, gpu, storage, devices, applications] = await Promise.allSettled([
    statfs(windows ? `${process.env.SystemDrive || 'C:'}\\` : '/'), defender(), auditBaseline(), gpuInventory(), diskInventory(), windowsDeviceIssues(), windowsApplicationIssues(),
  ]);
  const total = os.totalmem();
  const free = os.freemem();
  const disks = disk.status === 'fulfilled' ? { free: gib(disk.value.bavail * disk.value.bsize), total: gib(disk.value.blocks * disk.value.bsize) } : null;
  const events = [
    { area: 'Sistema', status: 'Concluído', detail: 'Informações de CPU e memória coletadas' },
    { area: 'Armazenamento', status: disks ? 'Concluído' : 'Indisponível', detail: disks ? 'Espaço do volume do sistema coletado' : 'Falha ao consultar o volume do sistema' },
    ...(protection.status === 'fulfilled' && protection.value.length ? protection.value : [{
      area: 'Segurança', status: 'Indisponível', detail: windows ? 'O sistema não retornou o estado do Defender' : 'O estado do Defender é consultado apenas no Windows',
    }]),
  ];
  events.push(...(baseline.status === 'fulfilled' ? baseline.value : [{
    area: 'Auditoria de configuração', status: 'Indisponível', detail: 'Agente local indisponível ou sem resposta',
  }]));
  if (gpu.status === 'fulfilled') events.push(gpu.value);
  events.push(...(storage.status === 'fulfilled' && storage.value.length ? storage.value : [{
    area: 'Discos', status: 'Indisponível', detail: 'O sistema não retornou o inventário de dispositivos',
  }]));
  if (devices.status === 'fulfilled') events.push(...devices.value);
  else events.push({ area: 'Dispositivos e drivers', status: 'Indisponível', detail: 'O Windows não retornou o estado dos dispositivos' });
  if (applications.status === 'fulfilled') events.push(...applications.value);
  else events.push({ area: 'Falhas de aplicativos', status: 'Indisponível', detail: 'O Windows não disponibilizou os eventos de aplicativos' });
  return { time: new Date().toISOString(), started, platform: `${os.type()} ${os.release()}`, cores: os.availableParallelism(), ram: { free: gib(free), total: gib(total), percent: Math.round(100 * (total - free) / total) }, disk: disks, protection: events[2].detail, events };
}

const files = new Map([
  ['/', ['index.html', 'text/html; charset=utf-8']],
  ['/guia-ia', ['guia-ia.html', 'text/html; charset=utf-8']],
  ['/ui.js', ['ui.js', 'text/javascript; charset=utf-8']],
  ['/triagem.js', ['triagem.js', 'text/javascript; charset=utf-8']],
  ['/bitcoin.js', ['bitcoin.js', 'text/javascript; charset=utf-8']],
]);
let running;
let scanRunning = false;
let backupRunning = false;
const snapshots = new Map();
createServer(async (req, res) => {
  res.setHeader('Cache-Control', 'no-store');
  res.setHeader('X-Content-Type-Options', 'nosniff');
  res.setHeader('Content-Security-Policy', "default-src 'self'; style-src 'unsafe-inline'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'");
  try {
    if (req.headers.host !== host) { res.writeHead(403).end(); return; }
    const exportMatch = req.url.match(/^\/relatorio\/([a-f0-9]{32})\.(docx|xlsx|pdf)$/);
    if (req.method === 'POST' && (req.url === '/diagnostico' || req.url === '/varredura' || req.url === '/varredura-arquivo' || req.url === '/rede' || req.url === '/backup' || exportMatch)) {
      if (req.headers.origin !== `http://${host}` || req.headers['x-diagnostico'] !== token) { res.writeHead(403).end(); return; }
      if (req.url === '/backup') {
        if (backupRunning) { res.writeHead(409).end(); return; }
        if (req.headers['content-type'] !== 'application/json') { res.writeHead(415).end(); return; }
        const length = Number(req.headers['content-length']);
        if (!Number.isSafeInteger(length) || length < 1 || length > 16384) { res.writeHead(413).end(); return; }
        backupRunning = true;
        res.setHeader('Content-Type', 'application/json');
        try {
          const chunks = [];
          let total = 0;
          for await (const chunk of req) {
            total += chunk.length;
            if (total > length) { res.writeHead(413).end(); return; }
            chunks.push(chunk);
          }
          if (total !== length) { res.writeHead(400).end(); return; }
          let input;
          try { input = JSON.parse(Buffer.concat(chunks)); } catch { res.writeHead(400).end(); return; }
          if (!input || typeof input !== 'object' || Array.isArray(input)) { res.writeHead(400).end(); return; }
          const { operation, repository, target, folder, id, password } = input;
          if ((target !== 'remote' && target !== 'local' && target !== undefined) || (target !== 'remote' && repository === '@s3')) { res.writeHead(400).end(); return; }
          if (typeof password !== 'string' || !password.length || password.length > 1024 || /[\x00\r\n]/.test(password)) { res.writeHead(400).end(); return; }
          const args = [operation, target === 'remote' ? '@s3' : repository];
          if (operation === 'salvar') args.push(folder);
          if (operation === 'restaurar') args.push(id, folder);
          if (!args.every((arg) => typeof arg === 'string' && arg.trim() && arg.length <= 4096 && !arg.includes('\0'))) { res.writeHead(400).end(); return; }
          const result = await runBackup(args, password, { allowS3: target === 'remote' });
          if (operation === 'listar') {
            const snapshots = JSON.parse(result.snapshots);
            res.end(JSON.stringify({ snapshots: (snapshots ?? []).slice(-50).map(({ id, time }) => ({ id, time })) }));
          } else res.end(JSON.stringify(result));
        } catch (error) {
          res.writeHead(422).end(JSON.stringify({ message: error.message }));
        } finally {
          backupRunning = false;
        }
        return;
      }
      if (req.url === '/varredura-arquivo') {
        if (process.platform !== 'linux') { res.writeHead(501).end(); return; }
        if (scanRunning) { res.writeHead(409).end(); return; }
        if (req.headers['content-type'] !== 'application/octet-stream') { res.writeHead(415).end(); return; }
        const length = Number(req.headers['content-length']);
        if (!Number.isSafeInteger(length) || length < 1 || length > maxScanBytes) { res.writeHead(413).end(); return; }
        scanRunning = true;
        try {
          const buffers = [];
          let received = 0;
          for await (const chunk of req) {
            received += chunk.length;
            if (received > length) { res.writeHead(413).end(); return; }
            buffers.push(chunk);
          }
          if (received !== length) { res.writeHead(400).end(); return; }
          const result = await scanBuffer(Buffer.concat(buffers, received));
          res.setHeader('Content-Type', 'application/json');
          res.end(JSON.stringify({ result }));
        } catch {
          res.writeHead(503).end();
        } finally {
          scanRunning = false;
        }
        return;
      }
      if (req.url === '/rede') {
        res.setHeader('Content-Type', 'application/json');
        res.end(JSON.stringify(await networkInventory()));
        return;
      }
      if (req.url === '/varredura') {
        if (!windows || !process.env.SystemRoot) { res.writeHead(501).end(); return; }
        if (scanRunning) { res.writeHead(409).end(); return; }
        scanRunning = true;
        try {
          await execute(powershellPath, ['-NoProfile', '-NonInteractive', '-Command', "$ErrorActionPreference='Stop'; Start-MpScan -ScanType QuickScan"], { timeout: 900000, maxBuffer: 32768, windowsHide: true });
          res.writeHead(204).end();
        } catch {
          res.writeHead(503).end();
        } finally {
          scanRunning = false;
        }
        return;
      }
      if (exportMatch) {
        const [, id, extension] = exportMatch;
        const report = snapshots.get(id);
        if (!report) { res.writeHead(404).end(); return; }
        const bytes = await renderReport(extension, report);
        res.setHeader('Content-Type', formats[extension]);
        res.setHeader('Content-Disposition', `attachment; filename="relatorio-diagnostico.${extension}"`);
        res.end(Buffer.from(bytes));
        return;
      }
      running ??= diagnose().then((report) => {
        const id = randomBytes(16).toString('hex');
        const snapshot = { ...report, id };
        snapshots.set(id, snapshot);
        if (snapshots.size > 16) snapshots.delete(snapshots.keys().next().value);
        return snapshot;
      }).finally(() => { running = undefined; });
      res.setHeader('Content-Type', 'application/json');
      res.end(JSON.stringify(await running));
      return;
    }
    const file = req.method === 'GET' && files.get(req.url);
    if (!file) { res.writeHead(404).end(); return; }
    const body = await readFile(new URL(file[0], import.meta.url), 'utf8');
    res.setHeader('Content-Type', file[1]);
    res.end(body.replace('__LOCAL_TOKEN__', token).replace('__BACKUP_S3_ENABLED__', String(isS3Configured())));
  } catch {
    res.writeHead(500).end('Falha no diagnóstico local.');
  }
}).listen(port, '127.0.0.1', () => console.log(`Central de diagnóstico em http://${host}`));
