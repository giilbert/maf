import { getDocsCategory } from "./helpers/content";
import { SideNav } from "./_components/side-nav";
import { Logo } from "@/components/logo";

export default async function DocsLayout(props: { children: React.ReactNode }) {
  const categories = await getDocsCategory();

  return (
    <div className="flex justify-center">
      <div className="p-6 lg:py-4 lg:px-12 space-y-8 w-full max-w-7xl">
        <div className="flex gap-2 items-center">
          <Logo size={24} />
          <h1 className="text-lg font-bold">MAF Documentation</h1>
        </div>

        <div className="gap-8 grid grid-cols-5">
          <SideNav categories={categories} />

          {props.children}
        </div>
      </div>
    </div>
  );
}
