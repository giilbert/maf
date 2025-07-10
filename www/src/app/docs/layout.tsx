import { getDocsCategory } from "./helpers/content";
import { SideNav } from "./_components/side-nav";

export default async function DocsLayout(props: { children: React.ReactNode }) {
  const categories = await getDocsCategory();

  return (
    <div className="flex justify-center">
      <div className="p-6 lg:py-4 lg:px-12 space-y-6 w-full max-w-7xl">
        <div className="gap-8 grid grid-cols-5">
          <div>
            <SideNav categories={categories} />
          </div>

          {props.children}
        </div>
      </div>
    </div>
  );
}
