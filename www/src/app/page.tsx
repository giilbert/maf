import { BundledLanguage } from "shiki";
import { Hero } from "./_components/hero";
import { SetupSection } from "./_components/sections";
import { CodeBlock } from "./docs/_components/mdx-components";
import { CODE_BLOCKS } from "./_components/scaffold-content";

export default function Home() {
  return (
    <div className="space-y-16">
      <Hero />

      <p className="my-[32rem]"></p>

      <SetupSection
        codeBlocks={Object.fromEntries(
          Object.keys(CODE_BLOCKS).map((key) => {
            const { language, content } =
              CODE_BLOCKS[key as keyof typeof CODE_BLOCKS];

            return [
              key,
              <CodeBlock key={key} lang={language as BundledLanguage}>
                {content.trim()}
              </CodeBlock>,
            ];
          })
        )}
      />

      <p className="mt-[80rem]">bottom</p>
    </div>
  );
}
