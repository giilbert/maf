import { useMutation, useQuery } from "@tanstack/react-query";
import { redirect } from "@tanstack/react-router";
import { API_SERVER_URL, APIError, get, post } from "./api";

export interface SessionInfo {
  id: string;
  name: string;
  email: string;
}

export const fetchSessionInfo = async (): Promise<SessionInfo | null> => {
  try {
    const res = await get<SessionInfo>("/api/v1/auth/session");
    return res;
  } catch (err) {
    if (err instanceof APIError && err.status === 401) return null;
    throw err;
  }
};

export const GET_SESSION_QUERY_KEY = ["auth", "getSession"];
export const useMaybeSession = () => {
  const sessionQuery = useQuery({
    queryKey: GET_SESSION_QUERY_KEY,
    queryFn: fetchSessionInfo,
  });
  return sessionQuery;
};

export const useSession = () => {
  const sessionQuery = useMaybeSession();
  if (!sessionQuery.data) throw redirect({ to: "/login" });
  return sessionQuery;
};

export const createLogInUrl = (opts: {
  provider: "google";
  redirect?: string;
}) => {
  return `${API_SERVER_URL}/api/v1/auth/login?redirect=${window.location.pathname}`;
};

export const useLogOut = () => {
  const signOutMutation = useMutation({
    mutationFn: () => post("/api/v1/auth/logout"),
    onSuccess: () => (window.location.href = "/"),
  });

  return signOutMutation;
};
