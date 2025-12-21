import { getRouteApi } from "@tanstack/react-router";

const Route = getRouteApi("/dynamic/$param");

export const DynamicPage: React.FC = () => {
  const params = Route.useParams();
  return <div>Dynamic Page: {params.param}</div>;
};
