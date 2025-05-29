import { redirect } from "next/navigation";

export default async function DocsPage() {
  redirect("/docs/getting-started/introduction");
}
