import { notFound } from "next/navigation";
import { getAllSlugs, getDocMeta, loadDocSource } from "../helpers/content";
import React from "react";
import { Heading } from "../_lib/toc";
import Link from "next/link";
import { MdxContentWrapper, renderMdx } from "../_components/mdx-renderer";

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

        <MdxContentWrapper>{content}</MdxContentWrapper>
      </div>

      <div className="col-span-1 hidden xl:block">
        {headings.length > 0 && <TableOfContents headings={headings} />}
      </div>
    </>
  );
}

const TableOfContents: React.FC<{
  headings: Heading[];
}> = ({ headings }) => {
  return (
    <div className="sticky top-9 space-y-4">
      <h2 className="text-sm font-semibold">On this page</h2>

      <ul className="flex flex-col gap-2">
        {headings.map((heading) => (
          <li
            key={heading.slug}
            className="text-sm text-muted-foreground hover:text-foreground hover:underline transition-colors"
            style={{
              marginLeft: `${(heading.level - 2) * 0.5}rem`,
            }}
          >
            <Link href={`#${heading.slug}`}>{heading.title}</Link>
          </li>
        ))}
      </ul>
    </div>
  );
};

export async function generateStaticParams() {
  const slugs = await getAllSlugs();
  return slugs.map((slug) => ({ slug: slug.split("/") }));
}

export const dynamicParams = false;
