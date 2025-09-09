import { useRpc, useStore } from "@usemaf/react";

export const App: React.FC = () => {
  const { data } = useStore<number>("count", 0);
  const incr_mutation = useRpc<number>("increment_counter", 1);

  return (
    <>
      <h1>lets fuckin go bro</h1>
      <div className="card">
        <button onClick={incr_mutation.mutateAsync}>count is {data}</button>
      </div>
      <p className="read-the-docs">
        Click on the Ur Mom and Eat Farts logos to learn more
      </p>
    </>
  );
};
