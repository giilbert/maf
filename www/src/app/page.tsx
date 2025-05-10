import { Button } from "@/components/ui/button";
import Link from "next/link";
import Balancer from "react-wrap-balancer";
import {
  BlocksIcon,
  CloudIcon,
  LockIcon,
  LucideArrowUpRightFromSquare,
  LucideIcon,
  RocketIcon,
} from "lucide-react";

export default function Home() {
  return (
    <div className="p-6 lg:pt-12 pb-8 md:px-16 xl:px-24 h-screen w-screen space-y-6 lg:space-y-8 flex flex-col">
      <Navbar />

      <div className="space-y-8 xl:space-y-0 lg:grid grid-cols-4 xl:grid-cols-5 gap-2 md:gap-8 h-full flex flex-col">
        <div className="col-span-2 space-y-12 flex flex-col">
          <div className="space-y-6 bg-neutral-200 px-8 pt-6 pb-8 relative h-full -ml-8 -mr-6 md:mr-0 flex flex-col justify-center">
            <div className="hidden md:block w-full h-full absolute bg-neutral-300 -z-10 top-4 left-4"></div>

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

          <div className="flex gap-2 items-center flex-col sm:flex-row">
            <Button size="lg" className="w-full">
              Get Started
            </Button>
          </div>
        </div>

        <div className="h-full col-span-2 xl:col-span-3 border border-dashed border-neutral-700 flex items-center justify-center">
          <p className="text-2xl">TODO: insert demo app here</p>
        </div>
      </div>
    </div>
  );
}

const FeatureCard: React.FC<{
  title: string;
  icon: LucideIcon;
  description: string;
}> = ({ title, description, icon: IconComponent }) => {
  return (
    <div className="flex flex-col gap-1 px-3 py-2 border-neutral-300 border bg-white cursor-pointer group hover:bg-neutral-100 transition-colors">
      <div className="flex items-center gap-3">
        <IconComponent size={24} />
        <h3 className="font-semibold group-hover:underline">{title}</h3>

        <LucideArrowUpRightFromSquare
          size={18}
          className="ml-auto group-hover:opacity-100 opacity-0 transition-opacity"
        />
      </div>
      <p className="text-neutral-600 text-sm">{description}</p>
    </div>
  );
};

const Navbar: React.FC = () => {
  return (
    <nav className="flex items-center gap-4">
      <p className="text-2xl font-bold">maf</p>

      <Link href="/docs" className="underline-offset-4 hover:underline ml-4">
        Docs
      </Link>

      <Link href="/docs" className="underline-offset-4 hover:underline">
        Examples
      </Link>
    </nav>
  );
};
