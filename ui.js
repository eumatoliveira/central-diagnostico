import { inspectFile, inspectLink } from './triagem.js';
import { compareRecipients } from './bitcoin.js';

const byId = (id) => document.getElementById(id);
const linkInput = byId('link-input');
const linkExpected = byId('link-expected');
const linkStatus = byId('link-status');
const linkFindings = byId('link-findings');
byId('link-review').addEventListener('input', () => {
  linkStatus.textContent = '';
  linkFindings.replaceChildren();
});
byId('link-review').addEventListener('submit', (event) => {
  event.preventDefault();
  linkFindings.replaceChildren();
  try {
    const { host, warnings } = inspectLink(linkInput.value, linkExpected.value);
    linkStatus.textContent = warnings.length ? `Confira antes de abrir. O domínio é ${host}.` : `O domínio é ${host}. Nenhum dos sinais básicos analisados apareceu.`;
    for (const text of [...warnings, 'Esta conferência não verifica a reputação do site. Se o link pedir senha ou pagamento, abra o aplicativo ou site oficial por um caminho conhecido.']) {
      const item = document.createElement('li');
      item.textContent = text;
      linkFindings.append(item);
    }
  } catch (error) {
    linkStatus.textContent = error.message;
  }
});
const benchmark = byId('benchmark');
benchmark.addEventListener('click', async () => {
  const result = byId('benchmark-status');
  benchmark.disabled = true;
  result.textContent = 'Medindo desempenho…';
  try {
    const chunk = new Uint8Array(4 * 1024 * 1024);
    chunk.fill(0x5a);
    const start = performance.now();
    for (let round = 0; round < 8; round += 1) await crypto.subtle.digest('SHA-256', chunk);
    const elapsed = performance.now() - start;
    result.textContent = `32 MiB em ${Math.round(elapsed)} ms. Este resultado serve para comparar rodadas e não diagnostica defeitos sozinho.`;
  } catch {
    result.textContent = 'O navegador não conseguiu executar a medição.';
  } finally {
    benchmark.disabled = false;
  }
});
const bitcoinFields = [byId('btc-trusted'), byId('btc-destination')];
const bitcoinStatus = byId('btc-status');
byId('btc-compare').addEventListener('click', async () => {
  const values = bitcoinFields.map((input) => input.value);
  let result;
  try { result = await compareRecipients(...values); } catch {
    bitcoinStatus.textContent = 'A validação local não está disponível neste navegador.';
    return;
  }
  if (bitcoinFields.some((input, index) => input.value !== values[index])) return;
  bitcoinStatus.textContent = {
    igual: 'Os endereços são idênticos e passaram na verificação de checksum. Confirme o destinatário na sua carteira.',
    diferente: 'Os endereços são diferentes. Pare e confirme o destino por um canal confiável.',
    formato: 'Preencha com dois endereços públicos Bitcoin da rede principal, nos formatos aceitos.',
  }[result];
});
byId('btc-clear').addEventListener('click', () => {
  for (const input of bitcoinFields) input.value = '';
  bitcoinStatus.textContent = '';
  bitcoinFields[0].focus();
});
for (const input of bitcoinFields) input.addEventListener('input', () => { bitcoinStatus.textContent = ''; });
const run = byId('run');
const exportsSection = byId('exports');
const exportStatus = byId('export-status');
const headers = { 'X-Diagnostico': document.querySelector('meta[name="local-token"]').content };
const backupForm = byId('backup-form');
const backupOperation = byId('backup-operation');
const backupFolder = byId('backup-folder');
const backupId = byId('backup-id');
const backupPassword = byId('backup-password');
const backupTarget = byId('backup-target');
const backupRepository = byId('backup-repository');
const remoteEnabled = document.querySelector('meta[name="backup-s3"]').content === 'true';
byId('backup-target-row').hidden = !remoteEnabled;
backupTarget.addEventListener('change', () => {
  const remote = remoteEnabled && backupTarget.value === 'remote';
  backupRepository.required = !remote;
  backupRepository.disabled = remote;
  byId('backup-repository-row').hidden = remote;
  byId('backup-snapshots').replaceChildren();
  byId('backup-status').textContent = '';
});
backupOperation.addEventListener('change', () => {
  const restoring = backupOperation.value === 'restaurar';
  backupFolder.required = restoring || backupOperation.value === 'salvar';
  backupId.required = restoring;
  backupFolder.disabled = !backupFolder.required;
  backupId.disabled = !backupId.required;
  byId('backup-folder-row').hidden = !backupFolder.required;
  byId('backup-id-row').hidden = !backupId.required;
  byId('backup-folder-label').textContent = restoring ? 'Pasta onde criar a restauração' : 'Pasta dos arquivos';
});
backupForm.addEventListener('submit', async (event) => {
  event.preventDefault();
  const button = backupForm.querySelector('button');
  const status = byId('backup-status');
  const list = byId('backup-snapshots');
  const operation = backupOperation.value;
  const remote = remoteEnabled && backupTarget.value === 'remote';
  if (remote && operation === 'salvar' && !window.confirm('Enviar o backup criptografado desta pasta ao armazenamento remoto configurado?')) return;
  const body = JSON.stringify({ operation, target: remote ? 'remote' : 'local', repository: remote ? undefined : backupRepository.value, folder: backupFolder.value, id: backupId.value, password: backupPassword.value });
  backupPassword.value = '';
  button.disabled = true;
  backupTarget.disabled = true;
  list.replaceChildren();
  status.textContent = 'Operação em andamento…';
  try {
    const response = await fetch('/backup', { method: 'POST', headers: { ...headers, 'Content-Type': 'application/json' }, body });
    if (response.status === 409) throw new Error('Já existe uma operação de backup em andamento.');
    const result = await response.json();
    if (Array.isArray(result.snapshots)) {
      for (const { id, time } of result.snapshots) {
        const item = document.createElement('li');
        const choose = document.createElement('button');
        choose.type = 'button';
        choose.textContent = `Selecionar cópia de ${new Date(time).toLocaleString('pt-BR')}`;
        choose.addEventListener('click', () => {
          backupOperation.value = 'restaurar';
          backupOperation.dispatchEvent(new Event('change'));
          backupId.value = id;
          backupFolder.value = '';
          backupFolder.focus();
          status.textContent = 'Cópia selecionada. Informe a pasta de destino e a senha para restaurar.';
        });
        item.append(choose);
        list.append(item);
      }
      status.textContent = list.children.length ? 'Escolha uma cópia para restaurar.' : 'Nenhuma cópia encontrada.';
    } else {
      status.textContent = result.destination ? `${result.message} Pasta: ${result.destination}` : result.message;
    }
  } catch (error) {
    status.textContent = error instanceof SyntaxError ? 'Verifique os dados informados e tente novamente.' : 'Não foi possível concluir a operação. Verifique se o servidor local está aberto e se há um backup em andamento.';
  } finally {
    button.disabled = false;
    backupTarget.disabled = false;
  }
});
const status = byId('status');
const number = new Intl.NumberFormat('pt-BR', { maximumFractionDigits: 1 });
let report;
const scan = byId('scan');
const scanStatus = byId('scan-status');
const network = byId('network');
network.addEventListener('click', async () => {
  const status = byId('network-status');
  const results = byId('network-results');
  network.disabled = true;
  status.textContent = 'Consultando interfaces e rotas…';
  results.replaceChildren();
  try {
    const response = await fetch('/rede', { method: 'POST', headers });
    if (!response.ok) throw new Error('Consulta indisponível.');
    const probes = await response.json();
    for (const { title, output } of probes) {
      const heading = document.createElement('h3');
      const pre = document.createElement('pre');
      heading.textContent = title;
      pre.style.whiteSpace = 'pre-wrap';
      pre.textContent = output;
      results.append(heading, pre);
    }
    status.textContent = probes.length ? 'Consulta concluída.' : 'Sistema sem módulo de rede disponível.';
  } catch {
    status.textContent = 'Não foi possível consultar a rede local.';
  } finally {
    network.disabled = false;
  }
});

scan.addEventListener('click', async () => {
  if (!window.confirm('Iniciar a varredura rápida do Microsoft Defender neste computador?')) return;
  scan.disabled = true;
  scanStatus.textContent = 'Aguardando a resposta do Microsoft Defender…';
  try {
    const response = await fetch('/varredura', { method: 'POST', headers });
    if (response.ok) scanStatus.textContent = 'O Defender aceitou a solicitação. Consulte o Histórico de proteção do Windows para ver as detecções.';
    else if (response.status === 409) scanStatus.textContent = 'Já existe uma solicitação de varredura em andamento.';
    else scanStatus.textContent = 'O Defender não confirmou a solicitação. Confira o estado e o histórico na Segurança do Windows.';
  } catch {
    scanStatus.textContent = 'A conexão local caiu. Confira o estado da varredura na Segurança do Windows.';
  } finally {
    scan.disabled = false;
  }
});

byId('file-input').addEventListener('change', async ({ target }) => {
  const file = target.files[0];
  if (!file) return;
  const state = byId('file-status');
  const results = byId('file-results');
  results.replaceChildren();
  target.disabled = true;
  state.textContent = 'Revisando o arquivo localmente…';
  try {
    for (const finding of await inspectFile(file)) {
      const item = document.createElement('li');
      item.textContent = finding;
      results.append(item);
    }
    if (byId('clam-enabled').checked) {
      state.textContent = 'Aguardando o ClamAV…';
      const response = await fetch('/varredura-arquivo', {
        method: 'POST', headers: { ...headers, 'Content-Type': 'application/octet-stream' }, body: file,
      });
      const item = document.createElement('li');
      if (response.ok) {
        const { result } = await response.json();
        item.textContent = result === 'detectado' ? 'ClamAV detectou uma ameaça.' : 'ClamAV não encontrou uma ameaça nesta verificação.';
      } else {
        item.textContent = 'ClamAV não concluiu a verificação. Confira as assinaturas locais e tente novamente.';
      }
      results.append(item);
    }
    state.textContent = 'Revisão concluída.';
  } catch (error) {
    state.textContent = error.message;
  } finally {
    target.value = '';
    target.disabled = false;
  }
});

run.addEventListener('click', async () => {
  run.disabled = true;
  status.textContent = 'Verificando o computador…';
  try {
    const response = await fetch('/diagnostico', { method: 'POST', headers });
    if (!response.ok) throw new Error('Diagnóstico indisponível');
    report = await response.json();
    scan.hidden = !report.platform.startsWith('Windows_NT');
    byId('clam-option').hidden = !report.platform.startsWith('Linux');
    byId('ram').textContent = `${number.format(report.ram.free)} de ${number.format(report.ram.total)} GiB livres`;
    byId('ram-meter').value = report.ram.percent;
    byId('ram-meter').hidden = false;
    const disk = report.disk;
    byId('disk').textContent = disk ? `${number.format(disk.free)} de ${number.format(disk.total)} GiB disponíveis` : 'Consulta indisponível';
    byId('disk-meter').hidden = !disk?.total;
    if (disk?.total) byId('disk-meter').value = 100 * (1 - disk.free / disk.total);
    byId('protection').textContent = report.protection;
    byId('system').textContent = `${report.platform} · ${report.cores} processadores lógicos disponíveis`;
    byId('events').replaceChildren(...report.events.map(({ area, status, detail }) => {
      const row = document.createElement('li');
      row.textContent = `${area} — ${status}. ${detail}.`;
      return row;
    }));
    const partial = report.events.some((event) => event.status === 'Indisponível');
    const review = report.events.some((event) => event.status === 'Requer revisão');
    const realtime = report.events.find((event) => event.area === 'Proteção em tempo real');
    const nextStep = byId('next-step');
    if (realtime?.status === 'Requer revisão') {
      nextStep.textContent = 'No menu Iniciar, procure Segurança do Windows e abra Proteção contra vírus e ameaças. Confira qual antivírus está protegendo este computador. Se precisar de ajuda, compartilhe o relatório com alguém de confiança.';
    } else if (review || partial) {
      nextStep.textContent = 'Alguns itens precisam de atenção ou não puderam ser consultados. Abra os detalhes ou baixe o relatório para pedir ajuda.';
    } else {
      nextStep.textContent = 'A verificação não apontou itens para revisar nas consultas realizadas.';
    }
    nextStep.hidden = false;
    status.textContent = 'Verificação concluída. Veja o resultado abaixo.';
    exportsSection.hidden = false;
  } catch {
    status.textContent = 'A consulta falhou. Tente executar o diagnóstico novamente.';
  } finally {
    run.disabled = false;
  }
});

for (const button of exportsSection.querySelectorAll('button')) {
  button.addEventListener('click', async () => {
    const extension = button.dataset.format;
    const snapshot = report;
    button.disabled = true;
    exportStatus.textContent = 'Preparando relatório…';
    try {
      let blob;
      if (extension === 'json') {
        const { time, platform, ram, disk, events } = snapshot;
        blob = new Blob([JSON.stringify({ time, platform, ram, disk, events }, null, 2)], { type: 'application/json' });
      } else if (extension === 'txt') {
        const lines = ['Central de diagnóstico', new Date(snapshot.time).toLocaleString('pt-BR'), '', byId('system').textContent, `Memória — ${byId('ram').textContent}`, `Armazenamento — ${byId('disk').textContent}`, '', ...Array.from(byId('events').children, (row) => row.textContent)];
        blob = new Blob([lines.join('\n')], { type: 'text/plain;charset=utf-8' });
      } else {
        const response = await fetch(`/relatorio/${snapshot.id}.${extension}`, { method: 'POST', headers });
        if (!response.ok) throw new Error('Falha ao exportar');
        blob = await response.blob();
      }
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = `relatorio-diagnostico.${extension}`;
      link.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
      exportStatus.textContent = 'Relatório preparado para download.';
    } catch {
      exportStatus.textContent = 'Não foi possível exportar. Execute o diagnóstico e tente novamente.';
    } finally {
      button.disabled = false;
    }
  });
}
