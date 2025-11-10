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
  tabId?: string;
  tabValue?: string;
}

type ExtendedHeadingNode = MdNode &
  MdParent &
  MdHeading & {
    tabId?: string;
    tabValue?: string;
  };

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

type JsxAttribute = {
  type: "mdxJsxAttribute";
  name: string;
  value: string;
};

const extractTocTransformer: Transformer<Root> = (ast, _file) => {
  const headings: Heading[] = [];
  const usedSlugs = new Set<string>();
  const defaultTabSelection: Record<string, string> = {};

  visit(ast, "mdxJsxFlowElement", (node) => {
    if (node.name !== "Tabs") return CONTINUE;

    const docAttr = node.attributes.find(
      (attr) => attr.type === "mdxJsxAttribute" && attr.name === "docId"
    ) as JsxAttribute | undefined;

    if (!docAttr || !docAttr.value) {
      console.warn("Tabs component is missing docId attribute", node);
      return CONTINUE;
    }
    const tabsId = docAttr.value;

    defaultTabSelection[tabsId] =
      (
        node.attributes.find(
          (attr) =>
            attr.type === "mdxJsxAttribute" && attr.name === "defaultValue"
        ) as JsxAttribute | undefined
      )?.value || "";

    visit(node, "mdxJsxFlowElement", (childNode) => {
      if (childNode.name !== "TabsContent") return CONTINUE;

      const valueAttr = childNode.attributes.find(
        (attr) => attr.type === "mdxJsxAttribute" && attr.name === "value"
      ) as JsxAttribute | undefined;

      if (!valueAttr || !valueAttr.value) {
        console.warn("TabContent component is missing value attribute", node);
        return CONTINUE;
      }
      const tabValue = valueAttr.value;

      visit(childNode, "heading", (headingNode: ExtendedHeadingNode) => {
        headingNode.tabValue = tabValue;
        headingNode.tabId = tabsId;
        // console.log("Found heading inside TabContent", headingNode);
      });
    });
  });

  visit(ast, "heading", (node: ExtendedHeadingNode) => {
    const title = getHeadingTextMdast(node);

    if (node.depth < 2) return CONTINUE;

    headings.push({
      level: node.depth,
      slug: generateSlug(title, usedSlugs),
      title,
      tabId: node.tabId,
      tabValue: node.tabValue,
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
                {
                  type: "VariableDeclarator",
                  id: { type: "Identifier", name: "defaultTabSelection" },
                  init: {
                    type: "Literal",
                    value: JSON.stringify(defaultTabSelection),
                  },
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
