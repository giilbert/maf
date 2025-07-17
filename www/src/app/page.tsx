import { BundledLanguage } from "shiki";
import { Hero } from "./_components/hero";
import {
  // DeploySection,
  GetStartedSection,
  PrimitivesSection,
  SetupSection,
} from "./_components/sections";
import { CodeBlock } from "./docs/_components/mdx-components";
import { CODE_BLOCKS } from "./_components/scaffold-content";
import Link from "next/link";

export default function Home() {
  return (
    <div className="space-y-[40rem]">
      <Hero />

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

      <PrimitivesSection />
      {/* <DeploySection /> */}
      <GetStartedSection />

      <footer className="px-6 md:px-16 xl:px-24 space-y-4 pt-10 pb-40 border-t">
        <p className="text-3xl font-bold">mutation authority framework</p>
        <Link
          href="https://github.com/giilbert/maf"
          className="hover:underline underline-offset-2"
        >
          https://github.com/giilbert/maf
        </Link>
      </footer>
    </div>
  );
}
