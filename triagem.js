const executable = /\.(exe|msi|scr|com|bat|cmd|ps1|vbs|js|hta|lnk|iso)$/i;
const maxBytes = 24 * 1024 * 1024;

export function inspectLink(input, officialDomain = '') {
  const text = input.trim();
  if (text.length > 4096 || !/^https?:\/\//i.test(text) || /[\u0000-\u0020\u007f\\]/u.test(text)) {
    throw new Error('Cole um único link completo que comece com https:// ou http://.');
  }
  let url;
  try { url = new URL(text); } catch { throw new Error('Não foi possível ler este endereço.'); }
  const host = url.hostname.replace(/\.$/, '');
  const warnings = [];
  if (url.protocol === 'http:') warnings.push('A conexão usa HTTP, que não protege os dados da mesma forma que HTTPS.');
  if (url.username || url.password) warnings.push('O endereço contém um trecho antes do domínio que pode confundir quem lê. Confira o domínio exibido abaixo.');
  if (host.startsWith('[') || /^\d+(?:\.\d+){3}$/.test(host)) warnings.push('O endereço usa um número de rede em vez de um nome de site.');
  if (host.split('.').some((label) => label.startsWith('xn--'))) warnings.push('O nome usa caracteres internacionalizados. Alguns podem se parecer com letras de outro domínio.');
  if (url.port) warnings.push('O endereço usa uma porta fora do padrão para este tipo de conexão.');
  const reference = officialDomain.trim();
  if (reference) {
    if (reference.length > 253 || !/^[\p{L}\p{M}\d.-]+$/u.test(reference)) throw new Error('Informe somente o domínio conhecido, como exemplo.com.br.');
    let expected;
    try { expected = new URL(`https://${reference}`).hostname.replace(/\.$/, ''); } catch { throw new Error('Não foi possível ler o domínio conhecido.'); }
    if (!expected.includes('.') || /^\d+(?:\.\d+){3}$/.test(expected) || !expected.split('.').every((label) => /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/.test(label))) {
      throw new Error('Informe somente o domínio conhecido, como exemplo.com.br.');
    }
    if (host !== expected && !host.endsWith(`.${expected}`)) warnings.unshift(`O endereço aponta para ${host}, fora do domínio que você informou (${expected}).`);
  }
  return { host, warnings };
}

function emailSignals(text) {
  const separator = /\r?\n\r?\n/.exec(text);
  if (!separator) return ['Não foi possível identificar o cabeçalho da mensagem.'];
  const header = text.slice(0, separator.index).replace(/\r?\n[ \t]+/g, ' ');
  const domain = (name) => {
    const line = header.split(/\r?\n/).find((value) => value.toLowerCase().startsWith(`${name}:`));
    return line?.match(/@([a-z0-9.-]+)(?=[>\s,]|$)/i)?.[1].toLowerCase();
  };
  const sender = domain('from');
  const reply = domain('reply-to');
  const findings = [];
  if (sender && reply && sender !== reply) findings.push(`O domínio de resposta (${reply}) difere do remetente (${sender}).`);
  if (!sender) findings.push('O domínio do remetente não pôde ser identificado.');
  const urls = text.match(/https?:\/\/[^\s<>"']+/gi) ?? [];
  const hosts = new Set();
  for (const url of urls.slice(0, 1000)) {
    try { hosts.add(new URL(url).hostname); } catch { /* texto incompleto */ }
  }
  if (hosts.size) findings.push(`Domínios visíveis na mensagem: ${[...hosts].slice(0, 20).join(', ')}.`);
  if (urls.length > 1000 || hosts.size > 20) findings.push('A lista de links foi limitada.');
  findings.push('A análise lê texto visível. Partes MIME codificadas e autenticação do remetente exigem análise adicional.');
  return findings;
}

export async function inspectFile(file) {
  if (file.size > maxBytes) throw new Error('Selecione um arquivo de até 24 MiB.');
  const content = await file.arrayBuffer();
  const digest = await crypto.subtle.digest('SHA-256', content);
  const hash = Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('');
  const findings = [`SHA-256: ${hash}`];
  if (executable.test(file.name)) findings.push('A extensão pode iniciar código. Confira a origem antes de executar.');
  if (/\.eml$/i.test(file.name)) findings.push(...emailSignals(new TextDecoder().decode(content)));
  findings.push('Triagem preliminar. Para verificar malware, use o mecanismo de antivírus do sistema.');
  return findings;
}
