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
import { Logo } from "@/components/logo";

const FOOTER_LINK_CLASSES =
  "text-muted-foreground underline underline-offset-2 hover:text-foreground transition-colors";

export default function Home() {
  return (
    <div>
      <div className="space-y-[10rem]">
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
      </div>

      <GetStartedSection />

      <footer className="mx-6 md:mx-16 xl:mx-24 pt-10 pb-16 border-t border-neutral-800 flex flex-col gap-6 md:flex-row">
        <div className="md:space-y-4">
          <div className="space-y-1">
            <div className="flex gap-3">
              <Logo />
              <p className="text-2xl font-bold">Cobble</p>
            </div>

            <p className="text-muted-foreground">
              mutation authority framework
            </p>
          </div>

          <div className="hidden md:block">
            <PlatformIndicator />
          </div>
        </div>

        <div className="md:ml-auto space-y-2 flex flex-col md:items-end">
          <Link
            href="https://github.com/giilbert/cobble"
            className={FOOTER_LINK_CLASSES}
            target="_blank"
            rel="noopener noreferrer"
          >
            github.com/giilbert/cobble
          </Link>
          <Link
            href="https://www.npmjs.com/package/@usecobble/client"
            className={FOOTER_LINK_CLASSES}
            target="_blank"
            rel="noopener noreferrer"
          >
            npmjs.com/package/@usecobble/client
          </Link>
          <Link
            href="https://crates.io/crates/cobble"
            className={FOOTER_LINK_CLASSES}
            target="_blank"
            rel="noopener noreferrer"
          >
            crates.io/crates/cobble
          </Link>
        </div>

        <div className="block md:hidden">
          <PlatformIndicator />
        </div>
      </footer>
    </div>
  );
}

// TODO:
const PlatformIndicator: React.FC = () => {
  return (
    <div className="flex gap-2 items-center">
      <div className="w-3 h-3 bg-green-500 rounded-full" />
      <p className="text-sm text-muted-foreground">
        Cobble Platform is healthy
      </p>
    </div>
  );
};
