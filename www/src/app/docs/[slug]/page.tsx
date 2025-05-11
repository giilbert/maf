import { notFound } from "next/navigation";
import { getAllSlugs, getDocMeta, loadDocSource } from "../helpers/content";
import { Suspense } from "react";
import { MDXRemote } from "next-mdx-remote-client/rsc";

import { type BundledLanguage, codeToHtml } from "shiki";

export default async function Page({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;

  const meta = await getDocMeta(slug);
  const source = await loadDocSource(slug);
  if (!meta || !source) return notFound();

  return (
    <div className="space-y-2">
      <p className="text-muted-foreground">{meta.category}</p>

      <Suspense fallback={<div className="loading">Loading...</div>}>
        <div className="flex flex-col gap-3" suppressHydrationWarning>
          <MDXRemote
            source={source}
            components={{
              h1: (props) => (
                <h1 className="text-4xl font-bold mb-2" {...props} />
              ),
              h2: (props) => <h2 className="text-3xl font-bold" {...props} />,
              h3: (props) => (
                <h3 className="text-2xl font-semibold" {...props} />
              ),
              h4: (props) => (
                <h4 className="text-xl font-semibold" {...props} />
              ),
              h5: (props) => (
                <h5 className="text-lg font-semibold" {...props} />
              ),
              code: (props) => {
                const { className } = props;
                const lang = className?.replace(
                  "language-",
                  ""
                ) as BundledLanguage;

                return <CodeBlock lang={lang}>{props.children}</CodeBlock>;
              },
              pre: (props) => {
                return <pre className="border px-4 py-3">{props.children}</pre>;
              },
            }}
          />
        </div>
      </Suspense>
    </div>
  );
}

export async function generateStaticParams() {
  const slugs = await getAllSlugs();
  return slugs.map((slug) => ({ slug }));
}

export const dynamicParams = false;

const CodeBlock: React.FC<{
  children: string;
  lang: BundledLanguage;
}> = async (props) => {
  const out = await codeToHtml(props.children, {
    lang: props.lang,
    theme: "github-dark-default",
    structure: "inline",
  });

  return <span dangerouslySetInnerHTML={{ __html: out }} />;
};
