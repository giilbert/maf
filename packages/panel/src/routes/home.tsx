import { getRouteApi, Link } from "@tanstack/react-router";
import { useMaybeSession, useLogOut } from "../lib/auth";
import { Button } from "@/components/ui/button";

const Route = getRouteApi("/");

export const HomePage: React.FC = () => {
  const session = useMaybeSession();
  const logout = useLogOut();

  return (
    <div className="p-8">
      <h1 className="font-bold">Home</h1>
      {session.data ? (
        <>
          <p>logged in as {session.data?.email}</p>
          <Button onClick={() => logout.mutate()}>log out</Button>
        </>
      ) : (
        <Link to="/login">Login</Link>
      )}
    </div>
  );
};
