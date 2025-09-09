"use client";

import Link from "next/link";
import { getDocsCategory } from "../helpers/content";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/cn";
import { Logo } from "@/components/logo";

export const SideNav: React.FC<{
  categories: Awaited<ReturnType<typeof getDocsCategory>>;
}> = ({ categories }) => {
  return (
    <nav className="flex flex-col gap-2 fixed top-8 w-max">
      <div className="mb-2">
        <Link
          href="/docs/getting-started/introduction"
          className="flex gap-2 items-center"
        >
          <Logo size={24} />
          <h1 className="text-lg font-bold">MAF</h1>
        </Link>
      </div>

      <CategoriesRenderer categories={categories} />
    </nav>
  );
};

export const CategoriesRenderer: React.FC<{
  categories: Awaited<ReturnType<typeof getDocsCategory>>;
  onNavigate?: () => void;
}> = ({ categories, onNavigate }) => {
  const pathname = usePathname();
  const docsSlug = pathname.replace("/docs/", "");

  return (
    <div className="flex flex-col gap-2">
      {categories.map((category) => (
        <div key={category.name} className="space-y-1">
          <h2 className="text font-semibold">{category.name}</h2>
          <ul className="flex flex-col gap-1 ml-3">
            {category.docs.map((doc) => (
              <li key={doc.slug}>
                <Link
                  href={`/docs/${doc.slug}`}
                  onClick={() => onNavigate?.()}
                  className={cn(
                    "text-sm",
                    docsSlug === doc.slug
                      ? "text-primary"
                      : "text-muted-foreground hover:text-primary hover:underline"
                  )}
                >
                  {doc.title}
                </Link>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
};
