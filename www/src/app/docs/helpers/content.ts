import { glob } from "glob";
import { join } from "path";
import { promises as fs } from "fs";
import { z } from "zod";
import parseFrontmatter from "gray-matter";

const CONTENT_PATH = process.cwd() + "/src/content";

const docAttributesSchema = z.object({
  title: z.string(),
  category: z.string(),
  parent: z.string().optional(),
});

interface Doc extends z.infer<typeof docAttributesSchema> {
  slug: string;
}

const stripExtension = (path: string) => path.replace(/\.(md|mdx)?$/, "");

export const getAllDocPaths = async () => {
  const files = await glob("**/*.{md,mdx}", { cwd: CONTENT_PATH });
  return files.map((file) => file.replace("\\\\", "/"));
};

export const getDocMeta = async (slug: string) => {
  const path = await findDocPath(slug);
  if (!path) return null;

  const { data: attributesRaw } = parseFrontmatter(
    await fs.readFile(path, "utf-8")
  );

  const attributes = docAttributesSchema.parse(attributesRaw);

  return {
    slug,
    ...attributes,
  } satisfies Doc;
};

export const findDocPath = async (slug: string) => {
  const filePaths = await glob(`${join(CONTENT_PATH, slug)}.*`, {
    absolute: true,
  });

  if (filePaths.length === 0) return null;
  if (filePaths.length > 1) {
    throw new Error(
      `Multiple files found for slug "${slug}": ${filePaths.join(", ")}`
    );
  }

  return filePaths[0];
};

export const getAllSlugs = async () => {
  const paths = await getAllDocPaths();
  return paths.map(stripExtension);
};

export const getAllDocs = async () => {
  const files = await getAllDocPaths();

  const docs = await Promise.all(
    files.map(async (path) => getDocMeta(stripExtension(path)))
  );

  return docs.filter((doc) => doc !== null);
};

const indexSchema = z.array(
  z.object({
    name: z.string(),
    docs: z.array(z.string()),
  })
);

export const getDocsCategory = async () => {
  const rawIndex = JSON.parse(
    await fs.readFile(join(CONTENT_PATH, "index.json"), "utf-8")
  );
  const index = indexSchema.parse(rawIndex);

  return await Promise.all(
    index.map(async (category) => {
      const categoryDocs = await Promise.all(
        category.docs.map(async (docSlug) => {
          const doc = await getDocMeta(docSlug);

          if (!doc) throw new Error(`Doc not found: ${docSlug}`);

          return {
            slug: docSlug,
            title: doc.title,
            parent: doc.parent || null,
          };
        })
      );

      return {
        name: category.name,
        docs: categoryDocs
          .filter((doc) => doc.parent === null)
          .map((doc) => ({
            ...doc,
            children: categoryDocs.filter((d) => d.parent === doc.slug),
          })),
      };
    })
  );
};

export const loadDocSource = async (slug: string) => {
  const path = await findDocPath(slug);
  if (!path) return null;

  const { content } = parseFrontmatter(await fs.readFile(path, "utf-8"));

  return content;
};
