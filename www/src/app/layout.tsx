import type { Metadata } from "next";
import { Inter } from "next/font/google";
import { Provider } from "react-wrap-balancer";
import "./globals.css";

export const metadata: Metadata = {
  title: "Mutation Authority Framework",
  openGraph: {
    type: "website",
    title: "Mutation Authority Framework",
    description:
      "Cobble is a framework for building, deploying, and testing authoritative real-time applications.",
    images: ["https://cobble.gilbertz.me/cover.png"],
  },
};

const inter = Inter({ subsets: ["latin"] });

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className={`${inter.className} dark`}>
        <Provider>{children}</Provider>
      </body>
    </html>
  );
}
