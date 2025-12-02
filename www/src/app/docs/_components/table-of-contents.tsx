import Link from "next/link";
import { Heading } from "../_lib/toc";
import { useContext, useEffect, useMemo, useState } from "react";
import { DocsTabsContext } from "@/components/ui/tabs";
import { cn } from "@/lib/cn";

interface Breakpoint {
  id: string;
  scrollTop: number;
}

export const TableOfContents: React.FC<{
  headings: Heading[];
  hideTitle?: boolean;
}> = ({ headings, hideTitle }) => {
  const docsTabs = useContext(DocsTabsContext);
  const [selected, setSelected] = useState<string | null>(null);

  if (!docsTabs)
    throw new Error("TableOfContents must be used within a DocsTabsContext");

  const breakpoints: Breakpoint[] = useMemo(() => {
    const kv = Array.from(docsTabs.scrollTops.entries());
    kv.sort(([_aId, aScroll], [_bId, bScroll]) => aScroll - bScroll);
    return kv.map(([id, scrollTop]) => ({ id, scrollTop }));
  }, [docsTabs.scrollTops]);

  useEffect(() => {
    const onScroll = () => {
      const scrollTop = document.scrollingElement?.scrollTop;
      if (!scrollTop) return;

      const PADDING = 10;
      const breakIndex = breakpoints.findIndex(
        (b) => b.scrollTop - PADDING > scrollTop
      );
      if (breakIndex === -1 || breakIndex === 0) {
        setSelected(null);
      } else {
        const currentId = breakpoints[breakIndex - 1].id;
        setSelected(currentId);
      }
    };

    onScroll();
    window.addEventListener("scroll", onScroll);
    return () => {
      window.removeEventListener("scroll", onScroll);
    };
  }, [breakpoints]);

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
              className={cn(
                "text-sm text-muted-foreground hover:text-foreground hover:underline transition-colors",
                selected === heading.slug && "text-foreground"
              )}
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
