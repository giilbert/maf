import { getDocsCategory } from "./helpers/content";
import { SideNav } from "./_components/side-nav";
import { MobileNav } from "./_components/mobile-nav";

export default async function DocsLayout(props: { children: React.ReactNode }) {
  const categories = await getDocsCategory();

  return (
    <div className="flex justify-center">
      <div className="p-4 pt-16 lg:p-6 lg:py-4 lg:px-12 space-y-6 w-full max-w-7xl">
        <div className="gap-8 flex lg:grid lg:grid-cols-4 xl:grid-cols-5">
          <div className="hidden md:block min-w-40">
            <SideNav categories={categories} />
          </div>

          <MobileNav />

          {props.children}
        </div>
      </div>
    </div>
  );
}
