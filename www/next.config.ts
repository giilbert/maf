import { type NextConfig } from "next";

// Static file extensions to be served from /_panel/*
const STATIC_EXTENSIONS = ["js", "css"];

export default {
  pageExtensions: ["js", "jsx", "ts", "tsx", "md", "mdx"],
  // Route /~/* to a SPA at /_panel/*
  rewrites: async () => [
    ...STATIC_EXTENSIONS.map((ext) => ({
      source: `/~/:path*/:file.${ext}`,
      destination: `/_panel/:path*/:file.${ext}`,
    })),
    // Fallback to index.html for SPA routing
    {
      source: "/~/:path*",
      destination: "/_panel/index.html",
    },
  ],
} satisfies NextConfig;
