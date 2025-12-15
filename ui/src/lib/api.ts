import { get } from "svelte/store";
import { authHeader } from "./store";

class ApiError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ApiError";
  }
}

export async function api(url: string, method = "GET", body: any = null) {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };
  const auth = get(authHeader);
  if (auth) {
    headers["Authorization"] = auth;
  }

  const options: RequestInit = { method, headers };
  if (body) {
    options.body = JSON.stringify(body);
  }

  const res = await fetch(url, options);

  if (res.status === 401) {
    window.dispatchEvent(new CustomEvent("unauthorized"));
    throw new ApiError("Unauthorized");
  }

  if (!res.ok) {
    const text = await res.text();
    throw new ApiError(text);
  }

  const text = await res.text();
  return text ? JSON.parse(text) : null;
}
