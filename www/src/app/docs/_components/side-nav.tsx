"use client";

import Link from "next/link";
import { getDocsCategory } from "../helpers/content";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/cn";

export const SideNav: React.FC<{
  categories: Awaited<ReturnType<typeof getDocsCategory>>;
}> = ({ categories }) => {
  const pathname = usePathname();
  const docsSlug = pathname.replace("/docs/", "");

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
    </nav>
  );
};
