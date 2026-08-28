export const API_SERVER_URL = "http://localhost:1147"; // TODO:

export const post = <T>(path: string, opts?: { data: unknown }): Promise<T> => {
  const data = opts?.data ?? {};

  return fetch(`${API_SERVER_URL}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  }).then((res) => {
    // TODO: error handling
    if (!res.ok) throw new Error(`Request failed with status ${res.status}`);

    return res.json();
  });
};
