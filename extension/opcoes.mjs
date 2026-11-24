import { buildRules } from './regras.mjs';

const form = document.querySelector('form');
const domains = document.querySelector('#domains');
const save = form.querySelector('button');
const status = document.querySelector('#status');
const dnr = chrome.declarativeNetRequest;

try {
  const rules = await dnr.getDynamicRules();
  domains.value = rules.flatMap(rule => rule.condition.requestDomains).join('\n');
  domains.disabled = false;
  save.disabled = false;
} catch {
  status.textContent = 'Não foi possível abrir a lista de bloqueio.';
}

form.addEventListener('submit', async event => {
  event.preventDefault();
  save.disabled = true;
  try {
    const rules = buildRules(domains.value);
    const current = await dnr.getDynamicRules();
    await dnr.updateDynamicRules({ removeRuleIds: current.map(rule => rule.id), addRules: rules });
    status.textContent = `Lista salva. Domínios bloqueados: ${rules.length}.`;
  } catch (error) {
    status.textContent = error instanceof TypeError ? 'Digite apenas domínios válidos.' : error.message;
  } finally {
    save.disabled = false;
  }
});
