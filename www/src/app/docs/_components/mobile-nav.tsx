import { Logo } from "@/components/logo";

export const MobileNav: React.FC = () => {
  return (
    <nav className="fixed md:hidden top-0 left-0 w-screen p-2 bg-background border-b flex items-center">
      <Logo size={24} className="m-2" />
      <p className="font-bold">MAF</p>
    </nav>
  );
};
