import createMdx from "@next/mdx";

const withMdx = createMdx({
  extension: /\.(md|mdx)?$/,
});

export default withMdx({
  pageExtensions: ["js", "jsx", "ts", "tsx", "md", "mdx"],
});
