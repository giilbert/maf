export const API_SERVER_URL = "http://localhost:1147"; // TODO:

/**
 * An error returned by the MAF Platform API. This is basically just a wrapper
 * around a (HTTP status code, error response).
 */
export class APIError extends Error {
  constructor(
    public status: number,
    public errorResponse: unknown,
    message: string,
  ) {
    super(message);
  }
}

export const apiFetch = <T>(path: string, init?: RequestInit): Promise<T> => {
  return fetch(`${API_SERVER_URL}${path}`, {
    ...init,
    credentials: "include",
  }).then(async (res) => {
    if (!res.ok)
      throw new APIError(
        res.status,
        await res.json(),
        `request failed with status ${res.status}`,
      );

    return res.json();
  });
};

/**  */
export const get = <T>(path: string): Promise<T> => {
  return apiFetch<T>(path, { method: "GET" });
};

export const post = <T>(path: string, opts?: { data: unknown }): Promise<T> => {
  const data = opts?.data ?? {};

  return apiFetch<T>(path, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
};
