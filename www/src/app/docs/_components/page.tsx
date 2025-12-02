"use client";

import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { TableOfContents } from "./table-of-contents";
import { DocsTabsContext } from "@/components/ui/tabs";
import { Heading } from "../_lib/toc";
import { Suspense, useCallback, useState } from "react";

export const RenderPage: React.FC<{
  content: React.ReactNode;
  headings: Heading[];
  meta: { category: string };
  defaultTabSelection: Record<string, string>;
}> = ({ content, headings, meta, defaultTabSelection }) => {
  const [tabSelection, setTabSelection] = useState(() => {
    return new Map<string, string>(Object.entries(defaultTabSelection));
  });
  const [scrollTops, setScrollTops] = useState<Map<string, number>>(new Map());

  const updateTabSelection = useCallback((id: string, selection: string) => {
    setTabSelection((prev) => {
      const newMap = new Map(prev);
      newMap.set(id, selection);
      return newMap;
    });
  }, []);

  const setScrollTopCallback = useCallback((id: string, scrollTop: number) => {
    setScrollTops((prev) => {
      const newMap = new Map(prev);
      newMap.set(id, scrollTop);
      return newMap;
    });
  }, []);

  return (
    <DocsTabsContext.Provider
      value={{
        tabSelections: tabSelection,
        updateTabSelection,
        scrollTops: scrollTops,
        setScrollTop: setScrollTopCallback,
      }}
    >
      <div className="space-y-4 lg:col-span-3 mt-4 w-full min-w-0">
        <p className="text-muted-foreground">{meta.category}</p>

        {headings.length !== 0 && (
          <Accordion type="single" collapsible className="xl:hidden">
            <AccordionItem value="on-this-page">
              <AccordionTrigger className="bg-muted py-1.5 px-2.5 -mx-2.5">
                On This Page
              </AccordionTrigger>
              <AccordionContent className="mt-2">
                <TableOfContents headings={headings} hideTitle />
              </AccordionContent>
            </AccordionItem>
          </Accordion>
        )}

        <Suspense fallback={<></>}>
          <div className="flex flex-col gap-5 w-full" suppressHydrationWarning>
            {content}
          </div>
        </Suspense>
      </div>

      <div className="col-span-1 hidden xl:block">
        {headings.length > 0 && <TableOfContents headings={headings} />}
      </div>
    </DocsTabsContext.Provider>
  );
};
