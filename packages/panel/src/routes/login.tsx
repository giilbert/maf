import { getRouteApi } from "@tanstack/react-router";
import { API_SERVER_URL, post } from "../lib/api";
import { useMutation } from "@tanstack/react-query";

const Route = getRouteApi("/login");

export const LoginPage: React.FC = () => {
  return (
    <div>
      <h1>login</h1>

      <a href={`${API_SERVER_URL}/api/v1/auth/login`}>google</a>
    </div>
  );
};
