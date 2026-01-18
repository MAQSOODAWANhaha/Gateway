export class ApiError extends Error {
  status: number;
  payload?: unknown;

  constructor(message: string, status: number, payload?: unknown) {
    super(message);
    this.status = status;
    this.payload = payload;
  }
}

async function parseJsonSafe(resp: Response) {
  try {
    return await resp.json();
  } catch {
    return null;
  }
}

export async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const resp = await fetch(path, {
    headers: {
      "content-type": "application/json",
      ...(init?.headers ?? {})
    },
    ...init
  });

  if (!resp.ok) {
    const payload = await parseJsonSafe(resp);
    const message = payload?.message ?? resp.statusText;
    throw new ApiError(message || "请求失败", resp.status, payload);
  }

  if (resp.status === 204) {
    return null as T;
  }

  return (await resp.json()) as T;
}
