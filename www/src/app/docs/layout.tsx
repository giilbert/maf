import Link from "next/link";
import { getDocsCategory } from "./helpers/content";

export default function DocsLayout(props: { children: React.ReactNode }) {
  return (
    <div className="flex justify-center">
      <div className="p-6 lg:py-8 lg:px-12 space-y-4 w-full max-w-7xl">
        <h1 className="text-xl font-bold">MAF Documentation</h1>

        <div className="gap-8 grid grid-cols-5">
          <SideNav />

          <div className="col-span-3">{props.children}</div>
          <aside>
            <p>Aside</p>
          </aside>
        </div>
      </div>
    </div>
  );
}

const SideNav: React.FC = async () => {
  const categories = await getDocsCategory();

  return (
    <nav className="flex flex-col gap-2">
      {categories.map((category) => (
        <div key={category.name} className="space-y-1">
          <h2 className="text font-semibold">{category.name}</h2>
          <ul className="flex flex-col gap-1 ml-3">
            {category.docs.map((doc) => (
              <li key={doc.slug}>
                <Link
                  href={`/docs/${doc.slug}`}
                  className="text-sm text-muted-foreground hover:text-primary"
                >
                  {doc.title}
                </Link>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </nav>
  );
};
