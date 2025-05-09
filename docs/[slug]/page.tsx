import path from "node:path";
import { glob } from "glob";

export default async function DocPage(props: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await props.params;
  const { default: Doc } = await import(`@/content/docs/${slug}.mdx`);

  return (
    <div>
      <h1>Doc Page</h1>
      <p>Slug: {slug}</p>
      <Doc />
    </div>
  );
}

export async function generateStaticParams() {
  const contentDir = path.resolve(process.cwd(), "src/content/docs");
  const files = await glob("**/*.mdx", {
    cwd: contentDir,
    absolute: true,
    signal: AbortSignal.timeout(1000),
  });

  const params = files.map((file) => {
    const relativePath = path.relative(contentDir, file);
    return {
      slug: relativePath
        .replace(/\.mdx$/, "")
        .split(path.sep)
        .join("/"),
    };
  });

  console.log("!!!!!!!! params", params);

  return params;
}

export const dynamicParams = false;
