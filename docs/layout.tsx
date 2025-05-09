export default async function DocsLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div>
      <p>docs</p>
      {children}
    </div>
  );
}
