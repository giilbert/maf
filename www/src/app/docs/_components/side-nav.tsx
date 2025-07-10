"use client";

import Link from "next/link";
import { getDocsCategory } from "../helpers/content";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/cn";
import { Logo } from "@/components/logo";

export const SideNav: React.FC<{
  categories: Awaited<ReturnType<typeof getDocsCategory>>;
}> = ({ categories }) => {
  const pathname = usePathname();
  const docsSlug = pathname.replace("/docs/", "");

  return (
    <nav className="flex flex-col gap-2 fixed top-8">
      <div className="mb-2">
        <Link href="/docs" className="flex gap-2 items-center">
          <Logo size={24} />
          <h1 className="text-lg font-bold">MAF Documentation</h1>
        </Link>
      </div>

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
