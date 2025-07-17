import { Logo } from "@/components/logo";
import Link from "next/link";

export const Navbar: React.FC = () => {
  return (
    <nav className="flex items-center gap-4">
      <Logo hasText />

      <Link
        href="/docs/getting-started/introduction"
        className="underline-offset-4 hover:underline ml-4"
      >
        Docs
      </Link>
    </nav>
  );
};
