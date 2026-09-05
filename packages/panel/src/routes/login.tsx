import { getRouteApi } from "@tanstack/react-router";
import { API_SERVER_URL, post } from "../lib/api";
import { useMutation } from "@tanstack/react-query";
import { useSession } from "../lib/auth";

const Route = getRouteApi("/login");

export const LoginPage: React.FC = () => {
  const session = useSession();

  console.log("session", session.data);

  return (
    <div>
      <h1>login</h1>
      <p>signed in as {session.data?.email}</p>

      <a
        href={`${API_SERVER_URL}/api/v1/auth/login?redirect=${window.location.pathname}`}
      >
        google
      </a>
    </div>
  );
};
