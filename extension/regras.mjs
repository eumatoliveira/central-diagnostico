export function buildRules(input) {
  const domains = new Set();
  for (const line of input.split(/\r?\n/)) {
    const value = line.trim();
    if (!value) continue;
    if (!/^[\p{L}\p{M}0-9.-]+$/u.test(value)) throw new Error('Use somente domínios, um por linha.');
    const hostname = new URL('https://' + value).hostname.replace(/\.$/, '');
    if (hostname.length > 253 || !hostname.includes('.') ||
        !hostname.split('.').every(label => /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/.test(label)) ||
        !/[a-z]/.test(hostname.split('.').at(-1))) {
      throw new Error('Digite um domínio válido, como exemplo.test.');
    }
    domains.add(hostname);
    if (domains.size > 1000) throw new Error('A lista aceita até 1.000 domínios.');
  }
  return Array.from(domains, (domain, index) => ({
    id: index + 1,
    priority: 1,
    action: { type: 'redirect', redirect: { extensionPath: '/aviso.html' } },
    condition: { requestDomains: [domain], resourceTypes: ['main_frame'] },
  }));
}
