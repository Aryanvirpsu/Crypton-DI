export function b64url(buf: ArrayBuffer): string {
  const arr = new Uint8Array(buf);
  let str = "";
  for (let i = 0; i < arr.length; i++) {
    str += String.fromCharCode(arr[i]);
  }
  return btoa(str).replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
}

export function fromB64url(str: string): ArrayBuffer {
  const s = str.replace(/-/g, "+").replace(/_/g, "/");
  const raw = atob(s);
  const buf = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) buf[i] = raw.charCodeAt(i);
  return buf.buffer;
}
