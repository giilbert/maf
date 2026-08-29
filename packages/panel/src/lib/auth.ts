import { useQuery } from "@tanstack/react-query";
import { APIError, get } from "./api";

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

export const useSession = () => {
  const sessionQuery = useQuery({
    queryKey: ["auth", "getSession"],
    queryFn: fetchSessionInfo,
  });

  return sessionQuery;
};
