const base58 = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
const bech32 = 'qpzry9x8gf2tvdw0s3jn54khce6mua7l';
const generators = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];

async function base58Valid(address) {
  if (!/^[13][1-9A-HJ-NP-Za-km-z]{25,34}$/.test(address)) return false;
  let value = 0n;
  for (const character of address) value = value * 58n + BigInt(base58.indexOf(character));
  const decoded = [];
  while (value > 0n) {
    decoded.push(Number(value & 255n));
    value >>= 8n;
  }
  const leadingZeros = address.match(/^1*/)[0].length;
  const bytes = Uint8Array.from([...Array(leadingZeros).fill(0), ...decoded.reverse()]);
  if (bytes.length !== 25 || ![0, 5].includes(bytes[0])) return false;
  const hash = await crypto.subtle.digest('SHA-256', bytes.slice(0, 21));
  const checksum = new Uint8Array(await crypto.subtle.digest('SHA-256', hash)).slice(0, 4);
  return checksum.every((byte, index) => byte === bytes[21 + index]);
}

function segwitValid(address) {
  if (address.length > 90 || !/^bc1[ac-hj-np-z02-9]+$/.test(address)) return false;
  const symbols = [...address.slice(3)].map((character) => bech32.indexOf(character));
  if (symbols.length < 8 || symbols.includes(-1) || symbols[0] > 16) return false;
  const [version, ...payload] = symbols.slice(0, -6);
  let checksum = 1;
  for (const symbol of [3, 3, 0, 2, 3, ...symbols]) {
    const high = checksum >>> 25;
    checksum = ((checksum & 0x1ffffff) << 5) ^ symbol;
    generators.forEach((generator, index) => { if ((high >>> index) & 1) checksum ^= generator; });
  }
  if (checksum !== (version === 0 ? 1 : 0x2bc830a3)) return false;
  const length = Math.floor(payload.length * 5 / 8);
  const padding = payload.length * 5 % 8;
  if (padding >= 5 || (payload.at(-1) & ((1 << padding) - 1)) !== 0) return false;
  return version === 0 ? length === 20 || length === 32 : length >= 2 && length <= 40;
}

export async function compareRecipients(trusted, destination) {
  const addresses = [trusted.trim(), destination.trim()];
  const valid = await Promise.all(addresses.map((address) => address.startsWith('bc1') ? segwitValid(address) : base58Valid(address)));
  if (valid.includes(false)) return 'formato';
  return addresses[0] === addresses[1] ? 'igual' : 'diferente';
}
