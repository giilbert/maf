"use client";

import { gsap } from "gsap";
import { useGSAP } from "@gsap/react";
import { RefObject, useRef, useState } from "react";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { cn } from "@/lib/cn";
import {
  ArchiveIcon,
  ArrowLeftRightIcon,
  HardDriveIcon,
  LockIcon,
  SquareFunctionIcon,
  XIcon,
} from "lucide-react";
import {
  ClientScaffoldExamples,
  ServerScaffoldExamples,
} from "./scaffold-examples";
import Link from "next/link";
import { Button } from "@/components/ui/button";

gsap.registerPlugin(useGSAP);
gsap.registerPlugin(ScrollTrigger);

const Wrapper: React.FC<{
  ref?: RefObject<HTMLElement | null>;
  children: React.ReactNode;
  className?: string;
}> = ({ ref, className, children }) => {
  return (
    <section
      ref={ref}
      className={cn(
        "px-6 md:px-16 xl:px-24 min-h-screen w-full py-6 space-y-4",
        className
      )}
    >
      {children}
    </section>
  );
};

export const SetupSection: React.FC<{
  codeBlocks: Record<string, React.ReactNode>;
}> = ({ codeBlocks }) => {
  const leftRef = useRef<HTMLDivElement>(null);
  const rightRef = useRef<HTMLDivElement>(null);
  const textRef = useRef<HTMLParagraphElement>(null);
  const [shouldBlink, setShouldBlink] = useState(false);

  useGSAP(
    () => {
      const media = gsap.matchMedia();

      const tl = gsap.timeline({
        scrollTrigger: {
          trigger: leftRef.current,
          pin: gsap.utils.selector(leftRef.current)("#stick"),
          scrub: 0.5,
          toggleActions: "play none none reverse",
          end: window.innerWidth > 1280 ? "bottom+=550px bottom" : undefined,
          // markers: true,
        },
      });

      const updateText = (content: string) => {
        if (textRef.current) textRef.current.textContent = content;
      };

      const COMMAND = "$ cobble create";

      tl.call(setShouldBlink, [false], 0);
      tl.fromTo(
        "#cursor",
        { y: 0, opacity: 0 },
        { y: 0, opacity: 1, duration: 0.1, ease: "power2.out" },
        0
      );

      for (let i = 0; i < COMMAND.length + 1; i++) {
        const delay = i * 0.4; // Adjust the delay as needed
        tl.call(updateText, [COMMAND.slice(0, i)], delay);
      }

      tl.call(setShouldBlink, [true], COMMAND.length * 0.1);

      tl.fromTo(
        "#punch",
        { y: 100, opacity: 0 },
        { y: 0, opacity: 1, duration: 1, ease: "power2.out" }
      );

      media.add(
        {
          isMobile: "(min-width: 1280px)",
        },
        (ctx) => {
          if (!ctx.isMobile) {
            for (const el of gsap.utils.selector(leftRef.current)(
              "#no-doing > *"
            )) {
              tl.fromTo(
                el,
                { opacity: 0 },
                { opacity: 1, ease: "power2.out", duration: 8, delay: 1 }
              );
            }
          } else {
            tl.fromTo(
              "#no-doing",
              { opacity: 0, y: 50 },
              { opacity: 1, y: 0, ease: "power2.out", duration: 0.5 }
            );
          }
        }
      );
    },
    { scope: leftRef }
  );

  useGSAP(() => {}, { scope: rightRef });

  return (
    <div>
      <div className="flex justify-center py-20">
        <p className="text-neutral-600 text-lg">(keep scrolling)</p>
      </div>
      <Wrapper ref={leftRef} className="xl:flex xl:gap-4">
        <div className="pb-20 xl:pb-96 xl:h-full xl:w-1/2" id="stick">
          <div id="command" className="font-mono flex gap-1 items-center mb-8">
            <p ref={textRef} className="text-lg h-6" />
            <div
              id="cursor"
              className={cn(
                "w-2.5 h-5 bg-neutral-300",
                shouldBlink && "animate-blink"
              )}
            />
          </div>

          <div id="punch" className="space-y-4">
            <h2 className="text-4xl xl:text-6xl font-bold">
              It&apos;s as Simple as That.
            </h2>
            <p className="xl:text-lg">Create a realtime server in seconds.</p>

            <div
              id="no-doing"
              className="flex flex-col mt-8 text-muted-foreground"
            >
              <div className="flex items-center gap-2">
                <XIcon />
                <p>Fumbling with Socket.io and Express</p>
              </div>

              <div className="flex items-center gap-2">
                <XIcon />
                <p>Implementing authentication patterns</p>
              </div>

              <div className="flex items-center gap-2">
                <XIcon />
                <p>Managing state across multiple clients</p>
              </div>

              <div className="flex items-center gap-2">
                <XIcon />
                <p>Handling complex data structures</p>
              </div>

              <div className="flex items-center gap-2">
                <XIcon />
                <p>Scaling your application</p>
              </div>

              <p className="mt-8 text-foreground text-xl font-bold">
                No need to reinvent the wheel.
              </p>
            </div>
          </div>
        </div>

        <div className="xl:mt-[100rem] space-y-12 grow">
          <div className="space-y-2 h-[28rem] sm:h-[32rem] md:h-[36rem]">
            <StepDisplay number={1}>
              Scaffold project with{" "}
              <span className="font-mono bg-muted px-1.5 py-1 rounded">
                `cobble create`
              </span>
            </StepDisplay>

            <p className="italic text-muted-foreground">View scaffold for...</p>

            <ServerScaffoldExamples codeBlocks={codeBlocks} />
          </div>

          <div className="space-y-2.5">
            <StepDisplay number={2}>
              Start a server with{" "}
              <span className="font-mono bg-muted px-1.5 py-1 rounded">
                `cobble run`
              </span>
            </StepDisplay>

            <div className="font-mono p-4 bg-neutral-900 text-xs sm:text-sm md:text-base">
              <p>
                <span className="font-bold text-green-500">INFO</span>: Running
                build command `...` in ...
              </p>

              <br />
              <p>≽^•⩊•^≼ ──☆*:・₊※*・:*:｀♪:*:。*・☆*</p>
              <p>... (compiler magic) ...</p>
              <br />

              <p>
                <span className="font-bold text-green-500">INFO</span>: Build
                completed in 123.45ms
              </p>

              <p>
                <span className="font-bold text-green-500">INFO</span>: [dev]
                Loaded room from ...server.wasm
              </p>

              <p>
                <span className="font-bold text-green-500">INFO</span>:
                Development server listening on 1147
              </p>
            </div>
          </div>

          <div>
            <StepDisplay number={3}>
              Connect to the server with a Cobble client
            </StepDisplay>

            <ClientScaffoldExamples codeBlocks={codeBlocks} />
          </div>
        </div>
      </Wrapper>
    </div>
  );
};

const StepDisplay: React.FC<{
  number: number;
  children: React.ReactNode;
}> = ({ number, children }) => {
  return (
    <div className="flex items-center gap-3">
      <div className="text-xl font-bold w-8 h-8 bg-purple-600 flex items-center justify-center">
        {number}
      </div>
      <p className="text-lg">{children}</p>
    </div>
  );
};

export const PrimitivesSection: React.FC = () => {
  return (
    <Wrapper>
      <h2 className="text-4xl xl:text-6xl font-bold">Goodbye Boilerplate!</h2>
      <p className="xl:text-lg">
        Cobble comes with powerful pre-made building blocks to build your app.
      </p>

      <hr className="my-4" />

      <div className="flex gap-2 flex-wrap">
        <div className="border-4 border-green-700 px-6 py-4 space-y-2">
          <div className="flex gap-2 items-center">
            <ArchiveIcon size={32} />
            <h3 className="font-bold text-3xl">Stores</h3>
          </div>
          <p>
            Persistent, synchronized, and shared state with fine-grained
            controls.
          </p>
        </div>

        <div className="border-4 border-purple-700 px-6 py-4 space-y-2">
          <div className="flex gap-2 items-center">
            <SquareFunctionIcon size={32} />
            <h3 className="font-bold text-3xl">
              Remote Procedure Calls (RPCs)
            </h3>
          </div>
          <p>Realtime transactions that feel like local invocations.</p>
        </div>

        <div className="border-4 border-amber-600 px-6 py-4 space-y-2">
          <div className="flex gap-2 items-center">
            <ArrowLeftRightIcon size={32} />
            <h3 className="font-bold text-3xl">Channels</h3>
          </div>
          <p>Anyhow back and forth message passing.</p>
        </div>

        <div className="border-4 border-blue-500 px-6 py-4 space-y-2">
          <div className="flex gap-2 items-center">
            <HardDriveIcon size={32} />
            <h3 className="font-bold text-3xl">Rooms</h3>
          </div>
          <p>Easily manage users and keep state separated.</p>
        </div>

        <div className="border-4 border-red-500 px-6 py-4 space-y-2">
          <div className="flex gap-2 items-center">
            <LockIcon size={32} />
            <h3 className="font-bold text-3xl">Authentication</h3>
          </div>
          <p>
            Secure your app with built-in authentication and authorization
            patterns.
          </p>
        </div>
      </div>
    </Wrapper>
  );
};

export const DeploySection: React.FC = () => {
  return (
    <Wrapper>
      <h2 className="text-4xl xl:text-6xl font-bold">Cobble Platform</h2>
    </Wrapper>
  );
};

export const GetStartedSection: React.FC = () => {
  return (
    <div className="w-full px-4 flex items-center justify-center h-screen mb-0 flex-col gap-8">
      <div className="space-y-4">
        <p className="font-bold text-5xl">It&apos;s time for realtime.</p>
        <Link href="/docs/getting-started/quickstart">
          <Button className="w-full text-lg sm:text-xl md:text-2xl md:py-4">
            Get Started With Cobble
          </Button>
        </Link>
      </div>
    </div>
  );
};
