"use client";

import Link from "next/link";
import { getDocsCategory } from "../helpers/content";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/cn";
import { Logo } from "@/components/logo";
import { ChevronDownIcon, ChevronUpIcon } from "lucide-react";

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
    <div className="flex flex-col gap-4">
      {categories.map((category) => (
        <div key={category.name} className="space-y-1">
          <h2 className="text font-semibold">{category.name}</h2>
          <ul className="flex flex-col gap-2">
            {category.docs.map((doc) => {
              const isSelected = docsSlug === doc.slug;
              const isChildSelected = docsSlug.startsWith(doc.slug + "/");
              const hasChildren = doc.children.length > 0;

              return (
                <li key={doc.slug}>
                  <Link
                    href={`/docs/${doc.slug}`}
                    onClick={() => onNavigate?.()}
                    className={cn(
                      "text-sm flex items-center justify-between",
                      isSelected
                        ? "text-primary"
                        : "text-muted-foreground hover:text-primary hover:underline"
                    )}
                  >
                    {doc.title}
                    {hasChildren &&
                      (isSelected || isChildSelected ? (
                        <ChevronUpIcon size={18} />
                      ) : (
                        <ChevronDownIcon size={18} className="opacity-50" />
                      ))}
                  </Link>

                  {hasChildren && (isSelected || isChildSelected) && (
                    <ul className="flex flex-col gap-1 ml-4 mt-1">
                      {doc.children.map((child) => {
                        const isChildSelected = docsSlug === child.slug;
                        return (
                          <li key={child.slug}>
                            <Link
                              href={`/docs/${child.slug}`}
                              onClick={() => onNavigate?.()}
                              className={cn(
                                "text-sm",
                                isChildSelected
                                  ? "text-primary"
                                  : "text-muted-foreground hover:text-primary hover:underline"
                              )}
                            >
                              {child.title}
                            </Link>
                          </li>
                        );
                      })}
                    </ul>
                  )}
                </li>
              );
            })}
          </ul>
        </div>
      ))}
    </div>
  );
};
