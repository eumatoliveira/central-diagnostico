import { spawn } from 'node:child_process';
import { access, lstat, mkdtemp, readdir, realpath } from 'node:fs/promises';
import { constants } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const usage = `Uso:
  node backup/restic.mjs iniciar <repositorio-vazio>
  node backup/restic.mjs salvar <repositorio> <pasta-origem>
  node backup/restic.mjs listar <repositorio>
  node backup/restic.mjs verificar <repositorio>
  node backup/restic.mjs restaurar <repositorio> <id-completo> <pasta-pai>`;

const argumentCounts = new Map([
  ['iniciar', 1], ['salvar', 2], ['listar', 1], ['verificar', 1], ['restaurar', 3],
]);
const allowedAwsEnvironment = new Set([
  'AWS_ACCESS_KEY_ID', 'AWS_SECRET_ACCESS_KEY', 'AWS_SESSION_TOKEN', 'AWS_REGION', 'AWS_DEFAULT_REGION',
]);

function s3Repository() {
  const configured = process.env.CENTRAL_BACKUP_S3_REPOSITORY || '';
  const location = /^s3:(https:\/\/[a-z\d.[\]:-]+\/[a-z\d][a-z\d.-]{1,61}[a-z\d](?:\/[a-z\d._~-]+)*\/?)$/i.exec(configured);
  if (!location || location[1].split('/').some((segment) => segment === '.' || segment === '..')) {
    throw new Error('Configure CENTRAL_BACKUP_S3_REPOSITORY com s3:https://host/bucket ou s3:https://host/bucket/prefixo.');
  }
  const url = URL.parse(location[1]);
  if (!url?.hostname || url.username || url.password) {
    throw new Error('O endereço S3 configurado é inválido.');
  }
  if (!process.env.AWS_ACCESS_KEY_ID || !process.env.AWS_SECRET_ACCESS_KEY) {
    throw new Error('Configure AWS_ACCESS_KEY_ID e AWS_SECRET_ACCESS_KEY no ambiente do backup S3.');
  }
  return configured;
}

export function isS3Configured() {
  try {
    s3Repository();
    return true;
  } catch {
    return false;
  }
}

async function directory(value) {
  const localDrivePath = process.platform === 'win32' && /^[a-z]:[\\/](?![\\/])/i.test(value);
  const remoteOrQualified = /^[\\/]{2}/.test(value) || /^[a-z][a-z\d+.-]*:/i.test(value) && !localDrivePath;
  if (value.startsWith('-') || remoteOrQualified) {
    throw new Error('Informe o caminho de uma pasta local.');
  }
  const resolved = await realpath(path.resolve(value));
  if (/^[\\/]{2}/.test(resolved) || !(await lstat(resolved)).isDirectory()) {
    throw new Error('O caminho resolvido precisa ser uma pasta local.');
  }
  return resolved;
}

function contains(parent, child) {
  const relative = path.relative(parent, child);
  return relative === '' || (!path.isAbsolute(relative) && relative !== '..' && !relative.startsWith(`..${path.sep}`));
}

async function resticExecutable() {
  const name = process.platform === 'win32' ? 'restic.exe' : 'restic';
  for (const entry of (process.env.PATH || '').split(path.delimiter)) {
    const dir = entry.replace(/^"|"$/g, '');
    if (!path.isAbsolute(dir) || /^[\\/]{2}/.test(dir)) continue;
    try {
      const candidate = await realpath(path.join(dir, name));
      if (/^[\\/]{2}/.test(candidate) || !(await lstat(candidate)).isFile()) continue;
      await access(candidate, constants.X_OK);
      return candidate;
    } catch { /* continue verificando os diretórios configurados */ }
  }
  throw new Error('Instale o restic pelo canal oficial e adicione sua pasta ao PATH.');
}

async function execute(repository, args, cwd, password) {
  const executable = await resticExecutable();
  const s3 = repository.startsWith('s3:');
  const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => {
    const name = key.toUpperCase();
    if (name === 'CENTRAL_BACKUP_S3_REPOSITORY') return false;
    if (name.startsWith('AWS_')) return s3 && allowedAwsEnvironment.has(name);
    return !name.startsWith('RESTIC_') || (password === undefined && name === 'RESTIC_PASSWORD');
  }));
  if (password !== undefined) env.RESTIC_PASSWORD = password;
  return new Promise((resolve, reject) => {
    const capture = password !== undefined;
    const child = spawn(executable, ['--repo', repository, ...args], { cwd, env, stdio: capture ? ['ignore', 'pipe', 'ignore'] : 'inherit', shell: false, windowsHide: capture });
    const chunks = [];
    let size = 0;
    child.stdout?.on('data', (chunk) => {
      size += chunk.length;
      if (size <= 2 * 1024 * 1024) chunks.push(chunk);
    });
    child.once('error', (error) => reject(error.code === 'ENOENT'
      ? new Error('Instale o restic pelo canal oficial e deixe o executável disponível no PATH.')
      : error));
    child.once('close', (code, signal) => {
      if (code === 0 && size <= 2 * 1024 * 1024) resolve(Buffer.concat(chunks).toString());
      else if (code === 0) reject(new Error('A saída do backup excedeu o limite. Consulte o restic pelo terminal.'));
      else reject(new Error(`O restic encerrou com ${signal || `código ${code}`}.`));
    });
  });
}

export async function runBackup([action, ...values], password, { allowS3 = false } = {}) {
  if (argumentCounts.get(action) !== values.length || !values.every(Boolean)) throw new Error(usage);
  const remote = values[0] === '@s3';
  if (remote && !allowS3) throw new Error('O backup S3 não está habilitado nesta interface.');
  const repository = remote ? s3Repository() : await directory(values[0]);

  switch (action) {
    case 'iniciar':
      if (!remote && (await readdir(repository)).length) throw new Error('Escolha uma pasta de repositório vazia.');
      await execute(repository, ['init'], undefined, password);
      return { message: 'Repositório criado.' };
    case 'salvar': {
      const source = await directory(values[1]);
      if (!remote && (contains(repository, source) || contains(source, repository))) {
        throw new Error('A origem e o repositório precisam estar em árvores de pastas separadas.');
      }
      const filesystemOptions = process.platform === 'win32' ? [] : ['--one-file-system'];
      await execute(repository, ['backup', ...filesystemOptions, '--', '.'], source, password);
      return { message: 'Backup concluído.' };
    }
    case 'listar':
      return { snapshots: await execute(repository, ['snapshots', '--json'], undefined, password) };
    case 'verificar':
      await execute(repository, ['check', '--read-data'], undefined, password);
      return { message: 'Verificação concluída. O restic leu e conferiu os dados do repositório.' };
    case 'restaurar': {
      const id = values[1];
      if (!/^[a-f0-9]{64}$/i.test(id)) throw new Error('Informe o identificador completo de 64 caracteres. Consulte o README para listá-lo.');
      const parent = await directory(values[2]);
      if (!remote && contains(repository, parent)) throw new Error('A restauração precisa ficar fora do repositório.');
      const destination = await mkdtemp(path.join(parent, 'restauracao-'));
      if (password === undefined) console.log(`Pasta de restauração: ${destination}`);
      try {
        await execute(repository, ['restore', id, '--target', destination, '--verify'], undefined, password);
      } catch (error) {
        throw new Error(`Verificação da restauração falhou. A pasta ${destination} pode conter arquivos parciais. ${error.message}`);
      }
      return { message: 'Restauração concluída e conteúdo verificado pelo restic.', destination };
    }
  }
}

if (process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url) {
  try {
    const result = await runBackup(process.argv.slice(2), undefined, { allowS3: true });
    if (result.message) console.log(result.message);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
