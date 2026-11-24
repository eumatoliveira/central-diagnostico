export function compareRecipients(expected, displayed) {
  const first = expected.trim();
  const second = displayed.trim();
  const looksLikeAddress = (value) =>
    /^(?:[13][1-9A-HJ-NP-Za-km-z]{25,34}|bc1[qp][ac-hj-np-z02-9]{14,86}|BC1[QP][AC-HJ-NP-Z02-9]{14,86})$/.test(value);
  if (!looksLikeAddress(first) || !looksLikeAddress(second)) return 'invalid';
  const normalize = (address) => address.startsWith('BC1') ? address.toLowerCase() : address;
  return normalize(first) === normalize(second) ? 'match' : 'mismatch';
}

const form = globalThis.document?.querySelector('#bitcoin-form');
if (form) form.addEventListener('submit', (event) => {
  event.preventDefault();
  const { trusted, displayed } = form.elements;
  const result = compareRecipients(trusted.value, displayed.value);
  form.querySelector('output').textContent = {
    invalid: 'Confira se os dois campos contêm apenas endereços públicos Bitcoin da rede principal, sem links ou dados adicionais.',
    match: 'Os endereços digitados coincidem. Confirme o destinatário e o valor separadamente na sua carteira.',
    mismatch: 'Os endereços são diferentes. Interrompa o pagamento e confirme o destinatário por um canal que já conhece.',
  }[result];
});
