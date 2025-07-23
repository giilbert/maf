import { Button } from "@/components/ui/button";
import Balancer from "react-wrap-balancer";
import {
  BlocksIcon,
  CloudIcon,
  LockIcon,
  LucideIcon,
  RocketIcon,
} from "lucide-react";
import { DemoApp } from "./demo-app";
import { Navbar } from "./nav-bar";
import Link from "next/link";

export const Hero: React.FC = () => {
  return (
    <div className="p-6 lg:pt-12 pb-8 md:px-16 xl:px-24 lg:h-screen w-screen space-y-6 lg:space-y-8 flex flex-col md:max-h-screen">
      <Navbar />

      <div className="space-y-8 xl:space-y-0 lg:grid grid-cols-4 xl:grid-cols-5 gap-2 md:gap-8 h-fit lg:h-[calc(100vh-9rem)] flex flex-col">
        <div className="col-span-2 space-y-12 flex flex-col h-full">
          <div className="space-y-6 bg-background-300 px-8 py-12 relative h-fit lg:h-full -ml-8 -mr-6 md:mr-0 flex flex-col justify-center">
            <div className="hidden md:block w-full h-full absolute bg-background-500 -z-10 top-4 left-4"></div>

            <div className="space-y-2">
              <p className="font-mono">{"//"} mutation authority framework</p>
              <h1 className="font-extrabold text-5xl md:text-6xl xl:text-[4rem] leading-[0.9]">
                <Balancer>
                  Take The <span className="italic">Time</span> Out Of Realtime
                </Balancer>
              </h1>
            </div>

            <p className="text-lg md:text-xl">
              <Balancer>
                <span className="font-bold font-mono">MAF</span> is an
                authoritative realtime framework for writing simple, secure, and
                scalable apps.
              </Balancer>
            </p>
          </div>

          <div className="grid grid-cols-2 gap-1">
            <FeatureCard
              icon={LockIcon}
              title="Secure By Design"
              description="Server-side access control and data validation is core to MAF."
            />
            <FeatureCard
              icon={BlocksIcon}
              title="Realtime Primitives"
              description="Ready-to-use realtime tools so you can focus on actually building."
            />
            <FeatureCard
              icon={RocketIcon}
              title="Built to Scale"
              description="Engineered for performance and scalability, MAF is designed to grow."
            />
            <FeatureCard
              icon={CloudIcon}
              title="Cloud Deployments"
              description="Get a ready-to-go deployment within 30 seconds of creating an app."
            />
          </div>

          <div className="flex gap-2 items-center flex-col sm:flex-row w-full">
            <Link href="/docs/getting-started/quickstart" className="w-full">
              <Button size="lg" className="w-full">
                Get Started
              </Button>
            </Link>
          </div>
        </div>

        <div className="h-[calc(100vh-8rem)] lg:h-full col-span-2 xl:col-span-3 border border-dashed border-neutral-700 flex items-center justify-center">
          <DemoApp />
        </div>
      </div>
    </div>
  );
};

const FeatureCard: React.FC<{
  title: string;
  icon: LucideIcon;
  description: string;
}> = ({ title, description, icon: IconComponent }) => {
  return (
    <div className="flex flex-col gap-1 px-3 py-2 border bg-background cursor-pointer group hover:bg-background-300 transition-colors">
      <div className="flex sm:items-center gap-3 sm:flex-row flex-col">
        <IconComponent size={24} />
        <h3 className="font-semibold group-hover:underline">{title}</h3>
      </div>
      <p className="text-muted-foreground text-sm">{description}</p>
    </div>
  );
};
