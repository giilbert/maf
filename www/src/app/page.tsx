import { Hero } from "./_components/hero";
import { SetupSection } from "./_components/sections";

export default function Home() {
  return (
    <div className="space-y-16">
      <Hero />

      <p className="my-[32rem]"></p>

      <SetupSection />

      <p className="mt-[80rem]">bottom</p>
    </div>
  );
}
