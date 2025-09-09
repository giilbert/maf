import { getDocsCategory } from "./helpers/content";
import { SideNav } from "./_components/side-nav";
import { MobileNav } from "./_components/mobile-nav";

export default async function DocsLayout(props: { children: React.ReactNode }) {
  const categories = await getDocsCategory();

  return (
    <div className="flex justify-center">
      <div className="p-4 pt-10 sm:pt-4 lg:p-6 lg:py-4 lg:px-12 space-y-6 w-full max-w-[85rem]">
        <div className="gap-4 md:gap-8 flex lg:grid lg:grid-cols-4 xl:grid-cols-5">
          <div className="hidden sm:block min-w-32 md:min-w-40">
            <SideNav categories={categories} />
          </div>

          <MobileNav categories={categories} />

          {props.children}
        </div>
      </div>
    </div>
  );
}
