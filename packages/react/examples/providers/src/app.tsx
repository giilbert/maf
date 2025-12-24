import { useStore, useMafClient } from "@usemaf/react";

export const App: React.FC = () => {
  const { data } = useStore("count", 0);
  const maf = useMafClient();

  return (
    <>
      <p>The count is {data}</p>
      <button
        onClick={() => {
          maf.rpc("increment_counter", 1);
        }}
      >
        Click me to increment!
      </button>
    </>
  );
};
