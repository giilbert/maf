import { notFound } from "next/navigation";
import { getAllSlugs, getDocMeta, loadDocSource } from "../helpers/content";
import React from "react";
import { MdxContentWrapper, renderMdx } from "../_components/mdx-renderer";
import { TableOfContents } from "../_components/table-of-contents";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";

export default async function Page({
  params,
}: {
  params: Promise<{ slug: string[] }>;
}) {
  const slug = (await params).slug.join("/");

  const meta = await getDocMeta(slug);
  const source = await loadDocSource(slug);
  if (!meta || !source) return notFound();

  const { content, headings } = await renderMdx({ source });

  // console.log("headings", headings.map((h) => h.title).join(", "));

  return (
    <>
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

        <MdxContentWrapper>{content}</MdxContentWrapper>
      </div>

      <div className="col-span-1 hidden xl:block">
        {headings.length > 0 && <TableOfContents headings={headings} />}
      </div>
    </>
  );
}

export async function generateStaticParams() {
  const slugs = await getAllSlugs();
  return slugs.map((slug) => ({ slug: slug.split("/") }));
}

export const dynamicParams = false;
