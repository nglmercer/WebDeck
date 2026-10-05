export function id() {
  const b = new Uint8Array(16);
  crypto.getRandomValues(b);
  return Array.from(b, (b) => b.toString(16).padStart(2, '0')).join('');
}
