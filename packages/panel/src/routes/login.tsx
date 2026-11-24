import { getRouteApi } from "@tanstack/react-router";
import { createLogInUrl } from "../lib/auth";
import { Button } from "@/components/ui/button";

const Route = getRouteApi("/login");

export const LoginPage: React.FC = () => {
  return (
    <div className="p-8">
      <h1>login</h1>
      <Button
        render={<a href={createLogInUrl({ provider: "google" })}>google</a>}
      />
    </div>
  );
};
