import { writable } from "svelte/store";

export const authHeader = writable<string | null>(
  localStorage.getItem("rsspub_auth"),
);
export const isAuthenticated = writable<boolean>(
  !!localStorage.getItem("rsspub_auth"),
);

authHeader.subscribe((value) => {
  if (value) {
    localStorage.setItem("rsspub_auth", value);
    isAuthenticated.set(true);
  } else {
    localStorage.removeItem("rsspub_auth");
    isAuthenticated.set(false);
  }
});

export const feeds = writable<any[]>([]);
export const schedules = writable<any[]>([]);
export const downloads = writable<string[]>([]);
export const emailConfig = writable<any>(null);

export const isLoginVisible = writable<boolean>(false);
export const popup = writable<{
  visible: boolean;
  title: string;
  message: string;
  isError: boolean;
}>({
  visible: false,
  title: "",
  message: "",
  isError: false,
});
