import type { Metadata } from "next";
import { Inter } from "next/font/google";
import { Provider } from "react-wrap-balancer";
import "./globals.css";

export const metadata: Metadata = {
  title: "mutation authority framework",
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
