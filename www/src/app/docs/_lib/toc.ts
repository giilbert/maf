import { type Plugin, type Transformer } from "unified";
import { CONTINUE, visit } from "unist-util-visit";
import { convert as slugify } from "url-slug";
import type {
  Node as MdNode,
  Parent as MdParent,
  Heading as MdHeading,
} from "mdast";
import type { Root, ElementContent } from "hast";

export interface Heading {
  level: number;
  slug: string;
  title: string;
}

const generateSlug = (title: string, usedSlugs: Set<string>) => {
  const slug = slugify(title);

  if (usedSlugs.has(slug)) {
    let i = 1;
    while (usedSlugs.has(`${slug}-${i}`)) i++;

    const numberedSlug = `${slug}-${i}`;
    usedSlugs.add(numberedSlug);
    return numberedSlug;
  }

  usedSlugs.add(slug);
  return slug;
};

const getHeadingTextMdast = (node: MdNode & MdParent): string => {
  return node.children
    .filter((child) => "value" in child && typeof child.value === "string")
    .map((child) => (child as { value: string }).value)
    .join("");
};

const extractTocTransformer: Transformer<Root> = (ast, _file) => {
  const headings: Heading[] = [];
  const usedSlugs = new Set<string>();

  visit(ast, "heading", (node: MdNode & MdParent & MdHeading) => {
    const title = getHeadingTextMdast(node);

    if (node.depth < 2) return CONTINUE;

    headings.push({
      level: node.depth,
      slug: generateSlug(title, usedSlugs),
      title,
    });
  });

  ast.children.push({
    type: "mdxjsEsm",
    value: "",
    data: {
      estree: {
        type: "Program",
        sourceType: "module",
        body: [
          {
            type: "ExportNamedDeclaration",
            declaration: {
              type: "VariableDeclaration",
              kind: "const",
              declarations: [
                {
                  type: "VariableDeclarator",
                  id: { type: "Identifier", name: "headings" },
                  init: { type: "Literal", value: JSON.stringify(headings) },
                },
              ],
            },
            specifiers: [],
            attributes: [],
          },
        ],
      },
    },
  });
};

export const extractToc: Plugin<[], Root> = () => extractTocTransformer;

const HEADING_RE = /^(h[1-6])$/;

const getHeadingTextHast = (el: ElementContent): string => {
  if ("value" in el) return el.value;
  if ("children" in el) return el.children.map(getHeadingTextHast).join("");
  return "";
};

const headingIdTransformer: Transformer<Root> = (ast, _file) => {
  const usedSlugs = new Set<string>();

  visit(ast, "element", (node) => {
    if (!HEADING_RE.test(node.tagName)) return CONTINUE;
    node.properties.id = generateSlug(getHeadingTextHast(node), usedSlugs);
  });
};

export const customHeadingId: Plugin<[], Root> = () => headingIdTransformer;
