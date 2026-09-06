import { getRouteApi, Link } from "@tanstack/react-router";
import { useMaybeSession, useLogOut } from "../lib/auth";

const Route = getRouteApi("/");

export const HomePage: React.FC = () => {
  const session = useMaybeSession();
  const logout = useLogOut();

  return (
    <div>
      <h1>Home</h1>
      {session.data ? (
        <>
          <p>logged in as {session.data?.email}</p>
          <button onClick={() => logout.mutate()}>log out</button>
        </>
      ) : (
        <Link to="/login">Login</Link>
      )}
    </div>
  );
};
