const base = "/api/v1";

export const session = {
  get token() {
    return localStorage.getItem("token");
  },
  set token(v: string | null) {
    v ? localStorage.setItem("token", v) : localStorage.removeItem("token");
  },
};

export async function api<T>(path: string, options: RequestInit = {}): Promise<T> {
  const res = await fetch(base + path, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...(session.token ? { Authorization: `Bearer ${session.token}` } : {}),
      ...options.headers,
    },
  });
  if (!res.ok)
    throw new Error(
      (await res.json().catch(() => ({ detail: res.statusText }))).detail,
    );
  return res.status === 204 ? (undefined as T) : res.json();
}
