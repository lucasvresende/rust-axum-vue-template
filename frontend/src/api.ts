const base = "/api/v1";

/** Persist the bearer token across reloads; assigning null clears it. */
export const session = {
  get token() {
    return localStorage.getItem("token");
  },
  set token(v: string | null) {
    v ? localStorage.setItem("token", v) : localStorage.removeItem("token");
  },
};

/**
 * Send an API request with the stored bearer token and caller-supplied options.
 * Reject failed requests with the API detail; successful 204 responses return undefined.
 */
export async function api<T>(
  path: string,
  options: RequestInit = {},
): Promise<T> {
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
