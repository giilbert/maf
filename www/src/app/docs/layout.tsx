import { getDocsCategory } from "./helpers/content";
import { SideNav } from "./_components/side-nav";

export default async function DocsLayout(props: { children: React.ReactNode }) {
  const categories = await getDocsCategory();

  return (
    <div className="flex justify-center">
      <div className="p-6 lg:py-4 lg:px-12 space-y-8 w-full max-w-7xl">
        <h1 className="text-xl font-bold">MAF Documentation</h1>

        <div className="gap-8 grid grid-cols-5">
          <SideNav categories={categories} />

          <div className="col-span-3">{props.children}</div>
          <aside>
            <p>Aside</p>
          </aside>
        </div>
      </div>
    </div>
  );
}
