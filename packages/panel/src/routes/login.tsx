import { getRouteApi } from "@tanstack/react-router";
import { createLogInUrl } from "../lib/auth";

const Route = getRouteApi("/login");

export const LoginPage: React.FC = () => {
  return (
    <div>
      <h1>login</h1>
      <a href={createLogInUrl({ provider: "google" })}>with google</a>
    </div>
  );
};
