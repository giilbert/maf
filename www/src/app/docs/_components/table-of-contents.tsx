import Link from "next/link";
import { Heading } from "../_lib/toc";
import { useContext } from "react";
import { DocsTabsContext } from "@/components/ui/tabs";

export const TableOfContents: React.FC<{
  headings: Heading[];
  hideTitle?: boolean;
}> = ({ headings, hideTitle }) => {
  const docsTabs = useContext(DocsTabsContext);

  if (!docsTabs)
    throw new Error("TableOfContents must be used within a DocsTabsContext");

  const tabSelections = docsTabs.tabSelections;

  return (
    <div className="sticky top-9 space-y-4">
      {!hideTitle && <h2 className="text-sm font-semibold">On this page</h2>}

      <ul className="flex flex-col gap-2">
        {headings.map((heading) =>
          !heading.tabId ||
          heading.tabValue === tabSelections.get(heading.tabId) ? (
            <li
              key={heading.slug}
              className="text-sm text-muted-foreground hover:text-foreground hover:underline transition-colors"
              style={{
                marginLeft: `${(heading.level - 2) * 1}rem`,
              }}
            >
              <Link href={`#${heading.slug}`}>{heading.title}</Link>
            </li>
          ) : null
        )}
      </ul>
    </div>
  );
};
