"use client";

import { gsap } from "gsap";
import { useGSAP } from "@gsap/react";
import { RefObject, useRef, useState } from "react";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { cn } from "@/lib/cn";
import { XIcon } from "lucide-react";
import {
  JavaPlain,
  JavascriptOriginal,
  PythonOriginal,
  RustOriginal,
  TypescriptOriginal,
} from "devicons-react";

gsap.registerPlugin(useGSAP);
gsap.registerPlugin(ScrollTrigger);

const Wrapper: React.FC<{
  ref: RefObject<HTMLElement | null>;
  children: React.ReactNode;
}> = ({ ref, children }) => {
  return (
    <section
      ref={ref}
      className="max-w-7xl px-6 md:px-16 xl:px-24 min-h-screen w-full py-8"
    >
      {children}
    </section>
  );
};

export const SetupSection: React.FC = () => {
  const ref = useRef<HTMLElement>(null);
  const textRef = useRef<HTMLParagraphElement>(null);
  const [shouldBlink, setShouldBlink] = useState(false);

  useGSAP(
    () => {
      const tl = gsap.timeline({
        scrollTrigger: {
          trigger: ref.current,
          pin: ref.current,
          scrub: true,
          toggleActions: "play none none reverse",
          markers: true,
        },
      });

      const updateText = (content: string) => {
        if (textRef.current) textRef.current.textContent = content;
      };

      const COMMAND = "$ maf create";

      tl.call(setShouldBlink, [false], 0);
      tl.fromTo(
        "#cursor",
        { y: 0, opacity: 0 },
        { y: 0, opacity: 1, duration: 0.1, ease: "power2.out" },
        0
      );

      for (let i = 0; i < COMMAND.length + 1; i++) {
        const delay = i * 0.2; // Adjust the delay as needed
        tl.call(updateText, [COMMAND.slice(0, i)], delay);
      }

      tl.call(setShouldBlink, [true], COMMAND.length * 0.1);

      tl.fromTo(
        "#punch",
        { y: 100, opacity: 0 },
        { y: 0, opacity: 1, duration: 1, ease: "power2.out" },
        COMMAND.length * 0.1 + 0.5
      );

      tl.eventCallback("onComplete", () => console.log("animation complete"));
    },
    { scope: ref }
  );

  return (
    <Wrapper ref={ref}>
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
        <h2 className="text-6xl font-bold">It&apos;s as Simple as That.</h2>
        <p className="text-lg">Create a realtime server in seconds.</p>

        <div className="flex flex-col mt-8">
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

          <p className="mt-8">No need to reinvent the wheel.</p>
        </div>

        <div className="flex gap-1 flex-wrap">
          <JavascriptOriginal size={40} />
          <TypescriptOriginal size={40} />
          <RustOriginal size={40} className="bg-neutral-100 rounded-full" />
          <PythonOriginal size={40} />
          <JavaPlain size={40} />
        </div>
      </div>
    </Wrapper>
  );
};
