import { getRouteApi } from "@tanstack/react-router";

const Route = getRouteApi("/loader");

export const LoaderPage: React.FC = () => {
  const data = Route.useLoaderData();
  return <p>message: {data.message}</p>;
};
