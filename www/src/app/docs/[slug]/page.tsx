import { notFound } from "next/navigation";
import { getAllSlugs, getDocMeta, loadDocSource } from "../helpers/content";
import React, { Suspense } from "react";
import { evaluate } from "next-mdx-remote-client/rsc";
import { mdxComponents } from "../_components/mdx-components";
import { customHeadingId, extractToc, Heading } from "../_lib/toc";
import Link from "next/link";

export default async function Page({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;

  const meta = await getDocMeta(slug);
  const source = await loadDocSource(slug);
  if (!meta || !source) return notFound();

  const { content, mod } = await evaluate({
    source,
    components: mdxComponents,
    options: {
      mdxOptions: {
        remarkPlugins: [extractToc],
        rehypePlugins: [customHeadingId],
      },
    },
  });
  const headings: Heading[] =
    "headings" in mod ? JSON.parse(mod.headings as string) : [];
  console.log("headings", headings.map((h) => h.title).join(", "));

  return (
    <>
      <div className="space-y-2 col-span-3">
        <p className="text-muted-foreground">{meta.category}</p>

        <Suspense fallback={<div className="loading">Loading...</div>}>
          <div className="flex flex-col gap-3" suppressHydrationWarning>
            {content}
          </div>
        </Suspense>
      </div>

      <div className="col-span-1">
        {headings.length > 0 && <TableOfContents headings={headings} />}
      </div>
    </>
  );
}

const TableOfContents: React.FC<{
  headings: Heading[];
}> = ({ headings }) => {
  return (
    <div className="sticky top-0 space-y-4">
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
  return slugs.map((slug) => ({ slug }));
}

export const dynamicParams = false;
